//! The reading order of `spec/documents.md` 8: world, includes, layers, changes, checks,
//! names and direction, then the unrequested changes.

use crate::change::{self, Applied, Base, Change, Removal};
use crate::check::{self, not_yet, Checker, Rank};
use crate::ids;
use crate::path::{self, Walk};
use crate::read::{self, decode};
use crate::report::{At, Fault, Faults};
use crate::schema::{self, Rule, WORLD_KEYS};
use crate::source::{Role, Source, Sources, Span, SrcId};
use crate::tree::{Entry, Form, Item, Link, Table, Value};
use crate::world::World;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use via_diag::nearest::{nearest, Nearest};
use via_diag::{codes, Record, Subject, Through};

/// What a build asks of reading.
#[derive(Clone, Debug, Default)]
pub struct LoadOptions {
    /// `--lib` roots, in the order given.
    pub lib_roots: Vec<PathBuf>,
    /// The value of `VIA_LIB`, if set.
    pub via_lib: Option<OsString>,
    /// The shipped root: `<executable's folder>/../lib`.
    pub shipped_root: Option<PathBuf>,
    /// The requested changes, as written on the command line.
    pub changes: Vec<ChangeRequest>,
}

impl LoadOptions {
    /// Options with `VIA_LIB` read from the environment and the shipped root found from the
    /// running executable (`documents.md` 4).
    pub fn from_env(lib_roots: Vec<PathBuf>, changes: Vec<ChangeRequest>) -> LoadOptions {
        let shipped_root = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.join("..").join("lib")));
        LoadOptions {
            lib_roots,
            via_lib: std::env::var_os("VIA_LIB"),
            shipped_root,
            changes,
        }
    }
}

/// One `--change C`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeRequest {
    /// `C` as written.
    pub written: String,
    /// Its 1-based argument index, for faults located by argument.
    pub argument: u32,
}

impl ChangeRequest {
    /// A request.
    pub fn new(written: impl Into<String>, argument: u32) -> ChangeRequest {
        ChangeRequest {
            written: written.into(),
            argument,
        }
    }

    /// `C` holding a `.` or `/` is a change file's path, otherwise a change name (`build.md`).
    pub fn is_path(&self) -> bool {
        self.written.contains('.') || self.written.contains('/')
    }
}

/// The result of reading a world: the merged world when no step ended with an error, and every
/// record found.
#[derive(Debug)]
pub struct Loaded {
    /// The merged world, `None` after an error.
    pub world: Option<World>,
    /// Every diagnostic of reading, emitted, in the order found (`via_diag::finish` sorts).
    pub records: Vec<Record>,
}

/// Reads a world: the world file at `world_path` (command-line argument `world_argument`), its
/// libraries, layers and the requested changes, into the merged world.
pub fn load(world_path: &Path, world_argument: u32, opts: &LoadOptions) -> Loaded {
    let mut l = Loader {
        opts,
        sources: Sources::default(),
        faults: Faults::default(),
        world_file: String::new(),
        world_dir: PathBuf::new(),
        world_argument,
    };
    let world = l.run(world_path);
    // A finished world holds the sources; otherwise the loader still does.
    let sources = world.as_ref().map_or(&l.sources, |w| &w.sources);
    let records = l
        .faults
        .list
        .iter()
        .map(|f| f.emit(sources, &l.world_file))
        .collect();
    Loaded { world, records }
}

/// The declaration of a layer in the world file.
#[derive(Clone, Debug)]
struct LayerDecl {
    id: String,
    header: Span,
    file: Option<(String, Span)>,
    owner: Option<String>,
    rank: i64,
    /// False when `rank` was written wrong, so it takes part in no uniqueness check.
    rank_ok: bool,
}

struct Loader<'o> {
    opts: &'o LoadOptions,
    sources: Sources,
    faults: Faults,
    world_file: String,
    world_dir: PathBuf,
    world_argument: u32,
}

/// A namespace entry: where an id was declared first.
struct Declared {
    address: String,
    file: String,
    header: Span,
}

fn header_of(e: &Entry) -> Span {
    match &e.item.value {
        Value::Table(t) => t.header(),
        _ => e.key,
    }
}

fn set_chain(t: &mut Table, chain: &dyn Fn(Span) -> Vec<Link>) {
    for e in t.entries.values_mut() {
        e.chain = chain(e.key);
        if let Value::Table(sub) = &mut e.item.value {
            set_chain(sub, chain);
        }
    }
}

impl Loader<'_> {
    fn fault(&mut self, f: Fault) {
        self.faults.push(f);
    }

    fn arg(&self) -> At {
        At::Argument(self.world_argument)
    }

    fn run(&mut self, world_path: &Path) -> Option<World> {
        // Step 1: the world file, its top-level keys and its layers.
        let (src, mut root) = self.read_world(world_path)?;
        let layers = self.world_top(&root);
        if self.faults.has_error() {
            return None;
        }
        // Step 2: libraries.
        let mut base = Base {
            root: Table::new(root.header(), Form::Root),
            homes: BTreeMap::new(),
        };
        self.libraries(&root, &mut base);
        if self.faults.has_error() {
            return None;
        }
        // Step 3: layers, and the base.
        let ranks: BTreeMap<String, Rank> = layers
            .iter()
            .map(|l| {
                (
                    l.id.clone(),
                    Rank {
                        rank: l.rank,
                        owner: l.owner.clone(),
                    },
                )
            })
            .collect();
        let mut docs: Vec<(String, Table)> = Vec::new();
        let world_name = self.world_file.clone();
        let world_root = std::mem::replace(&mut root, Table::new(Span::new(src, 0, 0), Form::Root));
        docs.push(("world".to_string(), world_root));
        for l in &layers {
            if l.id == "world" {
                continue;
            }
            if let Some(t) = self.read_layer(l, &world_name, &layers) {
                docs.push((l.id.clone(), t));
            }
        }
        if self.faults.has_error() {
            return None;
        }
        let inline = self.form_base(&mut base, docs);
        if self.faults.has_error() {
            return None;
        }
        // Step 4: the requested changes.
        let (requested, listed) = self.requested(&inline)?;
        let mut applied: Vec<(String, Applied)> = Vec::new();
        for ch in &requested {
            let fs = change::check_body(ch);
            self.faults.extend(fs);
        }
        if self.faults.has_error() {
            return None;
        }
        for ch in &requested {
            let mut copy = base.clone();
            let a = change::apply(&mut copy, ch);
            self.faults.extend(a.faults.iter().cloned());
            applied.push((ch.name.clone(), a));
        }
        if self.faults.has_error() {
            return None;
        }
        let conflicts = change::conflicts(&applied);
        let stop = !conflicts.is_empty();
        self.faults.extend(conflicts);
        if stop {
            return None;
        }
        let mut removed: Vec<Removal> = Vec::new();
        let mut from_files: Vec<(String, String)> = Vec::new();
        for ch in &requested {
            let a = change::apply(&mut base, ch);
            removed.extend(a.removed);
            if ch.file {
                from_files.extend(a.added);
            }
        }
        // What a change file adds is declared in the lowest layer it names downward from (6.1).
        check::rehome(&mut base, &from_files, &ranks);
        // Step 5, the pack, is `assets.md`'s reader, run on the merged `pack`.
        // Step 6: the merged world's checks.
        let checked = Checker::new(&base.root).run();
        let step6: Vec<(String, At)> = checked
            .iter()
            .map(|f| (f.code_str().to_string(), f.at.clone()))
            .collect();
        self.faults.extend(checked);
        if self.faults.has_error() {
            return None;
        }
        // Step 7: names and direction.
        let dir = check::direction(&base.root, &base.homes, &ranks, &removed);
        let mut seen = step6;
        seen.extend(dir.iter().map(|f| (f.code_str().to_string(), f.at.clone())));
        self.faults.extend(dir);
        if self.faults.has_error() {
            return None;
        }
        // Imports that reach back to a world on the way (`composition.md` 9).
        self.import_cycles(&base.root, src);
        if self.faults.has_error() {
            return None;
        }
        // Unrequested inline changes, each alone on a copy of the merged world (6.1).
        for (name, (ch, _)) in &inline {
            if requested.iter().any(|r| &r.name == name) {
                continue;
            }
            self.idle(&base, ch, &ranks, &seen);
        }
        Some(World {
            sources: std::mem::take(&mut self.sources),
            world_file: self.world_file.clone(),
            base,
            ranks,
            changes: listed,
            removed,
        })
    }

    /// `import.cycle` (`composition.md` 9): each `[import.<id>]` whose file, followed through
    /// the imports of the worlds it reaches, comes back to a world already on the way. Only the
    /// `file` keys are read on the way; each world is checked whole when it is built.
    fn import_cycles(&mut self, root: &Table, src: SrcId) {
        let Some(imports) = root.table("import") else {
            return;
        };
        let start = canonical(&self.world_dir.join(&self.world_file));
        for (id, e) in imports.iter() {
            let Value::Table(t) = &e.item.value else {
                continue;
            };
            let Some(fe) = t.get("file") else { continue };
            let Value::Str(file) = &fe.item.value else {
                continue;
            };
            let mut stack = vec![start.clone()];
            let next = canonical(&self.world_dir.join(&file.text));
            if let Some(cycle) = import_loop(&next, &mut stack, src) {
                let shown: Vec<String> = cycle
                    .iter()
                    .map(|p| shown_path(&self.world_dir, p))
                    .collect();
                self.fault(
                    Fault::new(
                        codes::import::CYCLE,
                        Subject::entity("import", id),
                        At::Span(fe.item.span),
                        format!("worlds import each other: {}", shown.join(" -> ")),
                        "drop one import of the loop: a world cannot hold itself",
                    )
                    .with(
                        "files",
                        via_diag::Value::texts(shown.iter().map(String::as_str)),
                    )
                    .chain(&fe.chain),
                );
            }
        }
    }

    fn idle(
        &mut self,
        merged: &Base,
        ch: &Change,
        ranks: &BTreeMap<String, Rank>,
        seen: &[(String, At)],
    ) {
        let mut copy = merged.clone();
        let mut found = change::check_body(ch);
        let a = change::apply(&mut copy, ch);
        found.extend(a.faults);
        found.extend(Checker::new(&copy.root).run());
        found.extend(check::direction(&copy.root, &copy.homes, ranks, &a.removed));
        for f in found {
            if seen.contains(&(f.code_str().to_string(), f.at.clone())) {
                continue;
            }
            let inner = f.code_str().to_string();
            let mut g = Fault::new(
                codes::doc::IDLE_CHANGE,
                f.subject.clone(),
                f.at.clone(),
                format!(
                    "the change `{}`, not requested in this build, has a fault: {}",
                    ch.name, f.message
                ),
                f.hint.clone(),
            )
            .with("code", inner)
            .chain(&f.chain);
            g.related = f.related.clone();
            self.fault(g);
        }
    }

    // ---- Step 1 ----

    fn read_world(&mut self, world_path: &Path) -> Option<(SrcId, Table)> {
        let (base_dir, parts) = path::split_cli(world_path);
        let opened = match path::walk(&base_dir, &parts) {
            Ok(p) => p,
            Err(w) => {
                let f = self.walk_fault(w, &parts, self.arg(), Subject::Cli, "world file");
                self.fault(f);
                return None;
            }
        };
        let name = parts.last().cloned().unwrap_or_default();
        self.world_file = name.clone();
        self.world_dir = opened
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let text = self.read_text(&opened, &name, self.arg())?;
        let src = self.sources.add(Source::new(name, Role::World, text));
        let root = self.parse(src)?;
        Some((src, root))
    }

    fn read_text(&mut self, p: &Path, name: &str, at: At) -> Option<String> {
        let bytes = match std::fs::read(p) {
            Ok(b) => b,
            Err(e) => {
                self.fault(Fault::new(
                    codes::doc::FILE_UNREADABLE,
                    Subject::Cli,
                    at,
                    format!("`{name}` is found but cannot be read: {}", e.kind()),
                    "make the file readable",
                ));
                return None;
            }
        };
        match decode(bytes, name) {
            Ok(t) => Some(t),
            Err(f) => {
                self.fault(*f);
                None
            }
        }
    }

    fn parse(&mut self, src: SrcId) -> Option<Table> {
        let text = self.sources.get(src)?.text().to_string();
        match read::read(src, &text) {
            Ok(t) => Some(t),
            Err(fs) => {
                self.faults.extend(fs);
                None
            }
        }
    }

    fn walk_fault(&self, w: Walk, parts: &[String], at: At, subject: Subject, what: &str) -> Fault {
        match w {
            Walk::Case { index, on_disk } => {
                let written = parts.get(index).cloned().unwrap_or_default();
                Fault::new(
                    codes::doc::PATH_CASE,
                    subject,
                    at,
                    format!("the {what} part `{written}` is `{on_disk}` on disk; case must match on every system"),
                    format!("write `{on_disk}`"),
                )
            }
            Walk::Missing { index, entries } => {
                let written = parts.get(index).cloned().unwrap_or_default();
                let shown: String = parts[..=index.min(parts.len().saturating_sub(1))].join("/");
                let mut f = Fault::new(
                    codes::doc::FILE_MISSING,
                    subject,
                    at,
                    format!("the {what} `{shown}` is not found"),
                    String::new(),
                );
                match nearest(&written, entries.iter().map(String::as_str)) {
                    Nearest::Found(n) => {
                        f.hint = format!("write `{n}`");
                    }
                    other => f.hint = other.hint("correct the path"),
                }
                f
            }
        }
    }

    /// The world file's top-level keys and `[layer.*]` tables (step 1).
    fn world_top(&mut self, root: &Table) -> Vec<LayerDecl> {
        self.top_keys(root, true);
        for key in ["name", "extent"] {
            if root.get(key).is_none() {
                let takes = if key == "name" {
                    "a string"
                } else {
                    "4 lengths, `[x0, y0, x1, y1]`"
                };
                self.fault(Fault::new(
                    codes::doc::MISSING_KEY,
                    Subject::World,
                    At::World,
                    format!("the world file has no `{key}`"),
                    format!("write `{key}`, {takes}"),
                ));
            }
        }
        let mut layers = Vec::new();
        let mut world = LayerDecl {
            id: "world".into(),
            header: root.header(),
            file: None,
            owner: None,
            rank: 0,
            rank_ok: true,
        };
        let mut world_rank_span = None;
        if let Some(Value::Table(lt)) = root.value("layer") {
            for (id, e) in lt.iter() {
                let sub = Subject::entity("layer", id);
                if let Some(f) = ids::check(id, sub.clone(), At::Span(e.key), "layer id") {
                    self.fault(f);
                    continue;
                }
                let Value::Table(t) = &e.item.value else {
                    self.fault(Fault::new(
                        codes::doc::TYPE,
                        sub,
                        At::Span(e.key),
                        format!(
                            "`layer.{id}` is a table, and {} is written",
                            e.item.value.form_word()
                        ),
                        format!("write `[layer.{id}]` with `file`, `owner` and `rank`"),
                    ));
                    continue;
                };
                let decl = self.layer_decl(id, t);
                if id == "world" {
                    world.owner = decl.owner;
                    world.rank = decl.rank;
                    world.rank_ok = decl.rank_ok;
                    world.header = t.header();
                    world_rank_span = t.get("rank").map(|r| r.key);
                } else {
                    layers.push(decl);
                }
            }
        }
        let _ = world_rank_span;
        layers.push(world);
        layers.sort_by(|a, b| a.rank.cmp(&b.rank).then_with(|| a.id.cmp(&b.id)));
        // `doc.layer_rank`: ranks unique, `world`'s included.
        for pair in layers.windows(2) {
            if pair[0].rank == pair[1].rank && pair[0].rank_ok && pair[1].rank_ok {
                let (a, b) = if pair[1].id == "world" {
                    (&pair[1], &pair[0])
                } else {
                    (&pair[0], &pair[1])
                };
                self.faults.push(
                    Fault::new(
                        codes::doc::LAYER_RANK,
                        Subject::entity("layer", &b.id),
                        At::Span(b.header),
                        format!(
                            "the layers `{}` and `{}` both have rank {}; ranks are unique",
                            a.id, b.id, b.rank
                        ),
                        if a.id == "world" {
                            "give the layer another rank: the world file is rank 0 unless `[layer.world] rank` says otherwise".to_string()
                        } else {
                            format!("give `{}` or `{}` another rank", a.id, b.id)
                        },
                    )
                    .related(Subject::entity("layer", &a.id), At::Span(a.header), "the other layer of that rank"),
                );
            }
        }
        layers
    }

    fn layer_decl(&mut self, id: &str, t: &Table) -> LayerDecl {
        let world = id == "world";
        let ctx = check::Ctx::entity("layer", id, Vec::new());
        let listed: Vec<&str> = schema::listed(schema::Shape::Layer).collect();
        for (k, e) in t.iter() {
            let sub = ctx.subject_key(k);
            match schema::rule(schema::Shape::Layer, k) {
                Some(Rule::NotYet(m)) => self.fault(not_yet(sub, At::Span(e.key), k, m)),
                // A dropped word with its fixed hint (`documents.md` 7): `after`
                // (`composition.md` 6.5).
                Some(Rule::Fixed(h)) => self.fault(Fault::new(
                    codes::doc::UNKNOWN_KEY,
                    sub,
                    At::Span(e.key),
                    format!("`{k}` is not a key of `layer.{id}`"),
                    h,
                )),
                Some(Rule::Ok(_)) if world && k == "file" => self.fault(Fault::new(
                    codes::doc::UNKNOWN_KEY,
                    sub,
                    At::Span(e.key),
                    "`[layer.world]` takes no `file`: the world file is its file",
                    "remove `file`",
                )),
                Some(_) => {}
                None => {
                    let mut f = Fault::new(
                        codes::doc::UNKNOWN_KEY,
                        sub,
                        At::Span(e.key),
                        format!("`{k}` is not a key of `layer.{id}`"),
                        String::new(),
                    );
                    match nearest(k, listed.iter().copied()) {
                        Nearest::Found(n) => {
                            f.hint = format!("write `{n}`");
                            f = f.with("nearest", n);
                        }
                        other => f.hint = other.hint("remove the key"),
                    }
                    self.fault(f);
                }
            }
        }
        let type_fault = |this: &mut Self, k: &str, e: &Entry, takes: &str| {
            this.fault(Fault::new(
                codes::doc::TYPE,
                ctx.subject_key(k),
                At::Span(e.key),
                format!(
                    "`layer.{id}.{k}` takes {takes}, and {} is written",
                    e.item.value.form_word()
                ),
                format!("write {takes}"),
            ));
        };
        let file = match t.get("file") {
            Some(e) => match &e.item.value {
                Value::Str(s) => Some((s.text.clone(), e.key)),
                _ => {
                    type_fault(self, "file", e, "a string");
                    None
                }
            },
            None => None,
        };
        let owner = match t.get("owner") {
            Some(e) => match &e.item.value {
                Value::Str(s) => Some(s.text.clone()),
                _ => {
                    type_fault(self, "owner", e, "a string");
                    None
                }
            },
            None => None,
        };
        let (rank, rank_ok) = match t.get("rank") {
            Some(e) => match &e.item.value {
                Value::Num(n) if n.integer => (n.norm.parse::<i64>().unwrap_or(0), true),
                _ => {
                    type_fault(self, "rank", e, "an integer");
                    (0, false)
                }
            },
            None => (0, world),
        };
        if !world {
            for (key, takes) in [
                ("file", "a path relative to the world file"),
                ("owner", "free text naming who answers for the layer"),
                ("rank", "an integer, unique across layers"),
            ] {
                if t.get(key).is_none() {
                    self.fault(Fault::new(
                        codes::doc::MISSING_KEY,
                        Subject::entity("layer", id),
                        At::Span(t.header()),
                        format!("`layer.{id}` has no `{key}`"),
                        format!("write `{key}`, {takes}"),
                    ));
                }
            }
        }
        LayerDecl {
            id: id.to_string(),
            header: t.header(),
            file,
            owner,
            rank,
            rank_ok,
        }
    }

    /// The top-level keys of a world or layer file (section 1's rules on where things go).
    fn top_keys(&mut self, root: &Table, world: bool) {
        let mut listed: Vec<&str> = schema::TOP_TABLES.to_vec();
        if world {
            listed.extend(["name", "extent", "resolution", "seed", "include", "pack"]);
        }
        for (k, e) in root.iter() {
            let sub = Subject::World;
            let at = At::Span(e.key);
            let ks = k.as_str();
            if WORLD_KEYS.contains(&ks) || ks == "layer" {
                if !world {
                    self.fault(
                        Fault::new(
                            codes::doc::WORLD_KEY_IN_LAYER,
                            sub,
                            at,
                            format!("`{k}` is written in the world file only, not in a layer"),
                            format!("move `{k}` to the world file"),
                        )
                        .chain(&e.chain),
                    );
                } else if let Some(Rule::NotYet(m)) = schema::world_key(ks) {
                    self.fault(not_yet(sub, at, k, m).chain(&e.chain));
                }
                continue;
            }
            match ks {
                "change" => {}
                "kind" => self.fault(
                    Fault::new(
                        codes::doc::KIND_OUTSIDE_LIBRARY,
                        sub,
                        at,
                        "a kind is declared in a library's `kinds.toml`, not in a world or layer",
                        "move the `[kind.*]` tables to `lib/<library>/kinds.toml` and name the library in `include`",
                    )
                    .chain(&e.chain),
                ),
                "remove" | "unset" => self.fault(outside_change(sub, at, k, &e.chain)),
                _ => match schema::top_table(ks) {
                    Some(Rule::Ok(_)) => {}
                    Some(Rule::NotYet(m)) => self.fault(not_yet(sub, at, k, m).chain(&e.chain)),
                    Some(Rule::Fixed(h)) => self.fault(
                        Fault::new(codes::doc::UNKNOWN_KEY, sub, at, format!("`{k}` is not a word of a world"), h)
                            .chain(&e.chain),
                    ),
                    None => {
                        let mut f = Fault::new(
                            codes::doc::UNKNOWN_KEY,
                            sub,
                            at,
                            format!("`{k}` is not a key or table of a {}", if world { "world file" } else { "layer" }),
                            String::new(),
                        )
                        .chain(&e.chain);
                        match nearest(k, listed.iter().copied()) {
                            Nearest::Found(n) => {
                                f.hint = format!("write `{n}`");
                                f = f.with("nearest", n);
                            }
                            other => f.hint = other.hint("remove it"),
                        }
                        self.fault(f);
                    }
                },
            }
        }
    }

    // ---- Step 2 ----

    fn roots(&self) -> Vec<(String, PathBuf)> {
        let mut out = vec![("beside".to_string(), self.world_dir.join("lib"))];
        for (i, r) in self.opts.lib_roots.iter().enumerate() {
            out.push((format!("--lib {}", i + 1), r.clone()));
        }
        if let Some(v) = &self.opts.via_lib {
            let entries = std::env::split_paths(v).filter(|p| !p.as_os_str().is_empty());
            for (i, r) in entries.enumerate() {
                out.push((format!("VIA_LIB {}", i + 1), r));
            }
        }
        out.push((
            "shipped".to_string(),
            self.opts.shipped_root.clone().unwrap_or_default(),
        ));
        out
    }

    fn libraries(&mut self, world_root: &Table, base: &mut Base) {
        let mut st = LibState::default();
        for (name, span) in include_names(world_root) {
            self.library(&name, span, &mut st, base);
        }
    }

    fn locate(&mut self, name: &str, span: Span) -> Option<PathBuf> {
        let roots = self.roots();
        let mut found: Vec<(String, PathBuf)> = Vec::new();
        let mut case: Option<String> = None;
        let mut seen_names: BTreeSet<String> = BTreeSet::new();
        for (label, root) in &roots {
            if root.as_os_str().is_empty() || !root.is_dir() {
                continue;
            }
            match path::walk(root, &[name.to_string()]) {
                Ok(p) if p.is_dir() => found.push((label.clone(), p)),
                Ok(_) => {}
                Err(Walk::Case { on_disk, .. }) => {
                    case.get_or_insert(on_disk);
                }
                Err(Walk::Missing { entries, .. }) => {
                    seen_names.extend(entries.into_iter().filter(|e| ids::is_id(e)));
                }
            }
        }
        let sub = Subject::World;
        if found.is_empty() {
            if let Some(on_disk) = case {
                self.fault(Fault::new(
                    codes::doc::PATH_CASE,
                    sub,
                    At::Span(span),
                    format!("the library `{name}` is the folder `{on_disk}` on disk; case must match on every system"),
                    format!("rename the folder to `{name}`, or write `{on_disk}` if that is an id"),
                ));
                return None;
            }
            let searched: Vec<&str> = roots.iter().map(|(l, _)| l.as_str()).collect();
            let mut f = Fault::new(
                codes::doc::INCLUDE_MISSING,
                sub,
                At::Span(span),
                format!(
                    "no library root holds `{name}`; searched {}",
                    searched.join(", ")
                ),
                String::new(),
            );
            match nearest(name, seen_names.iter().map(String::as_str)) {
                Nearest::Found(n) => {
                    f.hint = format!("write `{n}`");
                    f = f.with("nearest", n);
                }
                other => {
                    f.hint = other
                        .hint("add the library folder to a root, or pass its root with `--lib`")
                }
            }
            self.fault(f);
            return None;
        }
        if found.len() > 1 {
            let others: Vec<&str> = found[1..].iter().map(|(l, _)| l.as_str()).collect();
            self.fault(Fault::new(
                codes::doc::INCLUDE_SHADOWED,
                sub,
                At::Span(span),
                format!(
                    "the library `{name}` is found in {} and in {}; {} is used",
                    found[0].0,
                    others.join(", "),
                    found[0].0
                ),
                "remove or rename the copy you do not mean, if it is not on purpose",
            ));
        }
        found.into_iter().next().map(|(_, p)| p)
    }

    fn library(&mut self, name: &str, span: Span, st: &mut LibState, base: &mut Base) {
        if let Some(f) = ids::check(name, Subject::World, At::Span(span), "library name") {
            self.fault(f);
            return;
        }
        if let Some(pos) = st.stack.iter().position(|n| n == name) {
            let mut lp: Vec<&str> = st.stack[pos..].iter().map(String::as_str).collect();
            lp.push(name);
            self.fault(Fault::new(
                codes::doc::INCLUDE_CYCLE,
                Subject::World,
                At::Span(span),
                format!("libraries include each other: {}", lp.join(" -> ")),
                format!(
                    "remove `{name}` from the `include` of `{}`",
                    st.stack.last().map_or(name, String::as_str)
                ),
            ));
            return;
        }
        if !st.done.insert(name.to_string()) {
            return;
        }
        let Some(dir) = self.locate(name, span) else {
            return;
        };
        let display = format!("lib:{name}/kinds.toml");
        let kinds_path = match path::walk(&dir, &["kinds.toml".to_string()]) {
            Ok(p) => p,
            Err(Walk::Case { on_disk, .. }) => {
                self.fault(Fault::new(
                    codes::doc::PATH_CASE,
                    Subject::World,
                    At::Span(span),
                    format!("the library `{name}` holds `{on_disk}`; the kinds file is `kinds.toml`, case included"),
                    "rename it to `kinds.toml`",
                ));
                return;
            }
            // A library with no kinds file adds no kinds (code only, R19b).
            Err(Walk::Missing { .. }) => return,
        };
        let Some(text) = self.read_text(&kinds_path, &display, At::Span(span)) else {
            return;
        };
        let src = self.sources.add(Source::new(
            display.clone(),
            Role::Library {
                id: name.to_string(),
            },
            text,
        ));
        let Some(mut root) = self.parse(src) else {
            return;
        };
        let lib = name.to_string();
        set_chain(&mut root, &|_| {
            vec![Link {
                through: Through::Include,
                name: lib.clone(),
                span,
                owner: None,
            }]
        });
        for (k, e) in root.iter() {
            let at = At::Span(e.key);
            match k.as_str() {
                "include" | "kind" => {}
                "change" => self.fault(Fault::new(
                    codes::doc::CHANGE_NESTED,
                    Subject::World,
                    at,
                    "a kinds file holds no `[change]`; changes are written in a world or a layer",
                    "move the change to the world file or a layer",
                )),
                "remove" | "unset" => self.fault(outside_change(Subject::World, at, k, &e.chain)),
                _ => {
                    let mut f = Fault::new(
                        codes::doc::UNKNOWN_KEY,
                        Subject::World,
                        at,
                        format!("`{k}` is not a key of a kinds file, which holds `include` and `[kind.*]`"),
                        String::new(),
                    );
                    match nearest(k, ["include", "kind"]) {
                        Nearest::Found(n) => {
                            f.hint = format!("write `{n}`");
                            f = f.with("nearest", n);
                        }
                        _ => f.hint = "a library has no top-level keys but `include` and `[kind.*]`; constants live in a kind's `let`".into(),
                    }
                    self.fault(f);
                }
            }
        }
        if let Some(Value::Table(kinds)) = root.value("kind") {
            for (id, e) in kinds.iter() {
                let sub = Subject::entity("kind", id);
                if let Some(f) = ids::check(id, sub.clone(), At::Span(e.key), "kind name") {
                    self.fault(f.chain(&e.chain));
                    continue;
                }
                if !matches!(e.item.value, Value::Table(_)) {
                    self.fault(Fault::new(
                        codes::doc::TYPE,
                        sub,
                        At::Span(e.key),
                        format!(
                            "`kind.{id}` is a table, and {} is written",
                            e.item.value.form_word()
                        ),
                        format!("write `[kind.{id}]` with its keys"),
                    ));
                    continue;
                }
                if let Some(prev) = st.kinds.get(id) {
                    self.fault(
                        Fault::new(
                            codes::identity::DUPLICATE_ID,
                            sub.clone(),
                            At::Span(header_of(e)),
                            format!(
                                "the kind `{id}` is declared in `{}` and in `{display}`",
                                prev.file
                            ),
                            "rename one of the kinds, or include only one of the libraries",
                        )
                        .related(
                            sub,
                            At::Span(prev.header),
                            format!("declared in `{}`", prev.file),
                        ),
                    );
                    continue;
                }
                st.kinds.insert(
                    id.clone(),
                    Declared {
                        address: format!("kind.{id}"),
                        file: display.clone(),
                        header: header_of(e),
                    },
                );
                if let Some(t) = top_mut(&mut base.root, "kind", e.key) {
                    t.insert(id.clone(), e.clone());
                }
                base.homes
                    .insert(("kind".into(), id.clone()), name.to_string());
            }
        }
        st.stack.push(name.to_string());
        for (inner, ispan) in include_names(&root) {
            self.library(&inner, ispan, st, base);
        }
        st.stack.pop();
    }

    // ---- Step 3 ----

    fn read_layer(&mut self, l: &LayerDecl, world_name: &str, all: &[LayerDecl]) -> Option<Table> {
        let (file, span) = l.file.clone()?;
        let sub = Subject::entity("layer", &l.id).with_keys(&via_diag::KeyPath::key("file"));
        let parts = match path::form(&file) {
            Ok(p) => p,
            Err(why) => {
                self.fault(Fault::new(
                    codes::doc::PATH_FORM,
                    sub,
                    At::Span(span),
                    format!("the layer file `{file}` is not a portable path: {why}"),
                    "write a relative path with `/`, each part printable ASCII",
                ));
                return None;
            }
        };
        let norm = path::join(&[], &parts);
        let twice = |o: &LayerDecl| {
            o.id != l.id
                && o.file
                    .as_ref()
                    .and_then(|(f, _)| path::form(f).ok())
                    .is_some_and(|op| path::join(&[], &op) == norm)
        };
        if norm == [world_name.to_string()] || all.iter().any(twice) {
            self.fault(Fault::new(
                codes::doc::LAYER_FILE_TWICE,
                sub,
                At::Span(span),
                if norm == [world_name.to_string()] {
                    format!("the layer `{}` names the world file", l.id)
                } else {
                    format!(
                        "the layer `{}` names `{file}`, which another layer names too",
                        l.id
                    )
                },
                "give each layer its own file",
            ));
            return None;
        }
        let opened = match path::walk(&self.world_dir, &norm) {
            Ok(p) => p,
            Err(w) => {
                let f = self.walk_fault(w, &norm, At::Span(span), sub, "layer file");
                self.fault(f);
                return None;
            }
        };
        let display = norm.join("/");
        let text = self.read_text(&opened, &display, At::Span(span))?;
        let src = self.sources.add(Source::new(
            display,
            Role::Layer {
                id: l.id.clone(),
                owner: l.owner.clone(),
            },
            text,
        ));
        let mut root = self.parse(src)?;
        let (id, owner) = (l.id.clone(), l.owner.clone());
        set_chain(&mut root, &|s| {
            vec![Link {
                through: Through::Layer,
                name: id.clone(),
                span: s,
                owner: owner.clone(),
            }]
        });
        self.top_keys(&root, false);
        Some(root)
    }

    /// The base (section 5): every entity of every document, each declared once.
    fn form_base(
        &mut self,
        base: &mut Base,
        docs: Vec<(String, Table)>,
    ) -> BTreeMap<String, (Change, String)> {
        let mut world_ns: BTreeMap<String, Declared> = BTreeMap::new();
        let mut let_ns: BTreeMap<String, Declared> = BTreeMap::new();
        let mut inline: BTreeMap<String, (Change, String)> = BTreeMap::new();
        let mut change_ns: BTreeMap<String, Declared> = BTreeMap::new();
        for (home, root) in docs {
            let is_world = home == "world";
            let file = self.file_of(root.header().src);
            for (top, e) in root.entries {
                let ts = top.as_str();
                if WORLD_KEYS.contains(&ts) {
                    if is_world {
                        base.root.insert(top, e);
                    }
                    continue;
                }
                let Value::Table(t) = e.item.value else {
                    if schema::top_table(ts).is_some() || ts == "layer" || ts == "change" {
                        self.fault(Fault::new(
                            codes::doc::TYPE,
                            Subject::World,
                            At::Span(e.key),
                            format!("`{top}` is a table of entities keyed by id"),
                            format!("write `[{top}.<id>]` tables"),
                        ));
                    }
                    continue;
                };
                let table_ok = ts == "layer" && is_world
                    || ts == "change"
                    || matches!(schema::top_table(ts), Some(Rule::Ok(_)));
                if !table_ok {
                    continue;
                }
                let taken: std::collections::BTreeSet<String> = t.entries.keys().cloned().collect();
                for (id, ee) in t.entries {
                    if ts == "change" {
                        let sub = Subject::entity("change", &id);
                        if let Some(f) =
                            ids::check(&id, sub.clone(), At::Span(ee.key), "change name")
                        {
                            self.fault(f.chain(&ee.chain));
                            continue;
                        }
                        let header = header_of(&ee);
                        if let Some(prev) = change_ns.get(&id) {
                            self.fault(dup(&sub, header, &file, prev, &ee.chain));
                            continue;
                        }
                        change_ns.insert(
                            id.clone(),
                            Declared {
                                address: format!("change.{id}"),
                                file: file.clone(),
                                header,
                            },
                        );
                        let body = match ee.item.value {
                            Value::Table(b) => b,
                            other => {
                                self.fault(Fault::new(
                                    codes::doc::TYPE,
                                    sub,
                                    At::Span(ee.key),
                                    format!(
                                        "`change.{id}` is a table, and {} is written",
                                        other.form_word()
                                    ),
                                    format!("write `[change.{id}.<table>.<id>]`"),
                                ));
                                continue;
                            }
                        };
                        inline.insert(
                            id.clone(),
                            (
                                Change {
                                    name: id,
                                    body,
                                    home: home.clone(),
                                    file: false,
                                    header,
                                },
                                file.clone(),
                            ),
                        );
                        continue;
                    }
                    let sub = Subject::entity(ts, &id);
                    let what = if ts == "let" { "`let` name" } else { "id" };
                    if let Some(f) = ids::check(&id, sub.clone(), At::Span(ee.key), what) {
                        self.fault(f.chain(&ee.chain));
                        continue;
                    }
                    if id == "world" && ts != "layer" && ts != "let" {
                        self.fault(change::reserved(sub, &ee, &|c| taken.contains(c)));
                        continue;
                    }
                    // An import's id is the collection of its things' scoped subjects
                    // (`identity.md` 1): a collection's own name would read back as the
                    // parent's entity.
                    if ts == "import" && schema::reserved_import(&id) {
                        let free = change::free_name(&id, &|c| taken.contains(c));
                        self.fault(
                            Fault::new(
                                codes::identity::RESERVED,
                                sub,
                                At::Span(ee.key),
                                format!("the import id `{id}` is reserved: it names a collection of the world"),
                                format!("choose another id, such as `{free}`"),
                            )
                            .with("nearest", free.as_str())
                            .chain(&ee.chain),
                        );
                        continue;
                    }
                    if ts != "let" {
                        match &ee.item.value {
                            Value::Table(et) => self.no_removal(et, &sub, ts == "step"),
                            other => {
                                self.fault(Fault::new(
                                    codes::doc::TYPE,
                                    sub,
                                    At::Span(ee.key),
                                    format!(
                                        "`{ts}.{id}` is a table, and {} is written",
                                        other.form_word()
                                    ),
                                    format!("write `[{ts}.{id}]` with its keys"),
                                ));
                                continue;
                            }
                        }
                    }
                    let header = header_of(&ee);
                    let ns = if ts == "let" {
                        &mut let_ns
                    } else {
                        &mut world_ns
                    };
                    if let Some(prev) = ns.get(&id) {
                        let f = dup(&sub, header, &file, prev, &ee.chain);
                        self.fault(f);
                        continue;
                    }
                    if ts == "let" || schema::in_world_namespace(ts) {
                        ns.insert(
                            id.clone(),
                            Declared {
                                address: format!("{ts}.{id}"),
                                file: file.clone(),
                                header,
                            },
                        );
                    }
                    if let Some(tt) = top_mut(&mut base.root, ts, e.key) {
                        tt.insert(id.clone(), ee);
                    }
                    base.homes.insert((top.clone(), id), home.clone());
                }
            }
        }
        inline
    }

    fn file_of(&self, src: SrcId) -> String {
        self.sources.name(src).to_string()
    }

    /// `remove` and `unset` outside a change body (section 1), anywhere but inside `params`.
    fn no_removal(&mut self, t: &Table, sub: &Subject, step: bool) {
        for (k, e) in t.iter() {
            if k == "remove" || k == "unset" {
                let s = sub.clone().with_keys(&via_diag::KeyPath::key(k.clone()));
                self.fault(outside_change(s, At::Span(e.key), k, &e.chain));
            } else if let Value::Table(st) = &e.item.value {
                if step && k == "params" {
                    continue;
                }
                let s = sub.clone().with_keys(&via_diag::KeyPath::key(k.clone()));
                self.no_removal(st, &s, false);
            }
        }
    }

    // ---- Step 4 ----

    /// The requested changes, each once, and the `@changes` list.
    fn requested(
        &mut self,
        inline: &BTreeMap<String, (Change, String)>,
    ) -> Option<(Vec<Change>, Vec<String>)> {
        let mut out: Vec<Change> = Vec::new();
        let mut listed: BTreeSet<String> = BTreeSet::new();
        let mut files: BTreeMap<PathBuf, String> = BTreeMap::new();
        let mut by_name: BTreeMap<String, String> = BTreeMap::new();
        for req in &self.opts.changes {
            let at = At::Argument(req.argument);
            if !req.is_path() {
                if listed.contains(&req.written) {
                    continue;
                }
                match inline.get(&req.written) {
                    Some((ch, _)) => {
                        if let Some(other) = by_name.get(&ch.name) {
                            self.fault(Fault::new(
                                codes::identity::DUPLICATE_ID,
                                Subject::entity("change", &ch.name),
                                at,
                                format!("the change `{}` is requested by name and as the file `{other}`", ch.name),
                                "rename the change file, or request only one",
                            ));
                            continue;
                        }
                        by_name.insert(ch.name.clone(), ch.name.clone());
                        listed.insert(ch.name.clone());
                        out.push(ch.clone());
                    }
                    None => {
                        let mut f = Fault::new(
                            codes::doc::UNKNOWN_CHANGE,
                            Subject::Cli,
                            at,
                            format!("no change is named `{}`", req.written),
                            String::new(),
                        );
                        match nearest(&req.written, inline.keys().map(String::as_str)) {
                            Nearest::Found(n) => {
                                f.hint = format!("write `{n}`");
                                f = f.with("nearest", n);
                            }
                            other => f.hint = other.hint("write `[change.<name>]` in the world or a layer, or pass a change file's path"),
                        }
                        self.fault(f);
                    }
                }
                continue;
            }
            let Some((ch, display, canon)) = self.change_file(req) else {
                continue;
            };
            if files.contains_key(&canon) {
                continue;
            }
            files.insert(canon, display.clone());
            if by_name.contains_key(&ch.name)
                || inline.contains_key(&ch.name)
                    && self.opts.changes.iter().any(|r| r.written == ch.name)
            {
                self.fault(Fault::new(
                    codes::identity::DUPLICATE_ID,
                    Subject::entity("change", &ch.name),
                    at,
                    format!(
                        "the change `{}` is requested both as a name and as the file `{display}`",
                        ch.name
                    ),
                    "rename the change file, or request only one",
                ));
                continue;
            }
            by_name.insert(ch.name.clone(), display.clone());
            listed.insert(display);
            out.push(ch);
        }
        if self.faults.has_error() {
            return None;
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        let mut listed: Vec<String> = listed.into_iter().collect();
        listed.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        Some((out, listed))
    }

    fn change_file(&mut self, req: &ChangeRequest) -> Option<(Change, String, PathBuf)> {
        let at = At::Argument(req.argument);
        let p = Path::new(&req.written);
        let file_name = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(stem) = file_name.strip_suffix(".toml") else {
            self.fault(Fault::new(
                codes::doc::PATH_FORM,
                Subject::Cli,
                at,
                format!("the change file `{}` does not end in `.toml`", req.written),
                "name change files `<name>.toml`",
            ));
            return None;
        };
        if let Some(f) = ids::check(
            stem,
            Subject::entity("change", stem),
            at.clone(),
            "change file name",
        ) {
            self.fault(f);
            return None;
        }
        let (base_dir, parts) = path::split_cli(p);
        let opened = match path::walk(&base_dir, &parts) {
            Ok(o) => o,
            Err(w) => {
                let f = self.walk_fault(w, &parts, at, Subject::Cli, "change file");
                self.fault(f);
                return None;
            }
        };
        let canon = std::fs::canonicalize(&opened).unwrap_or_else(|_| opened.clone());
        let display = std::fs::canonicalize(&self.world_dir)
            .ok()
            .and_then(|wd| canon.strip_prefix(wd).ok().map(Path::to_path_buf))
            .map_or_else(
                || format!("change:{file_name}"),
                |rel| {
                    rel.components()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("/")
                },
            );
        let text = self.read_text(&opened, &display, at)?;
        let src = self.sources.add(Source::new(
            display.clone(),
            Role::ChangeFile {
                name: stem.to_string(),
            },
            text,
        ));
        let body = self.parse(src)?;
        let start = body.header();
        Some((
            Change {
                name: stem.to_string(),
                body,
                home: "world".into(),
                file: true,
                header: start,
            },
            display,
            canon,
        ))
    }
}

#[derive(Default)]
struct LibState {
    stack: Vec<String>,
    done: BTreeSet<String>,
    kinds: BTreeMap<String, Declared>,
}

fn include_names(root: &Table) -> Vec<(String, Span)> {
    match root.value("include") {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|it| match &it.value {
                Value::Str(s) => Some((s.text.clone(), it.span)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn top_mut<'t>(root: &'t mut Table, top: &str, span: Span) -> Option<&'t mut Table> {
    if root.get(top).is_none() {
        root.insert(
            top.to_string(),
            Entry {
                key: span,
                item: Item {
                    span,
                    value: Value::Table(Table::new(span, Form::Header)),
                },
                chain: Vec::new(),
            },
        );
    }
    root.table_mut(top)
}

fn outside_change(sub: Subject, at: At, k: &str, chain: &[Link]) -> Fault {
    Fault::new(
        codes::doc::REMOVE_OUTSIDE_CHANGE,
        sub,
        at,
        format!("`{k}` is written outside a change body; the base says what is, not what is not"),
        "delete the table or key itself, or write the removal in a `[change.<name>]`",
    )
    .chain(chain)
}

fn dup(sub: &Subject, header: Span, file: &str, prev: &Declared, chain: &[Link]) -> Fault {
    Fault::new(
        codes::identity::DUPLICATE_ID,
        sub.clone(),
        At::Span(header),
        format!(
            "`{sub}` is declared in `{file}`, and `{}` is declared in `{}`; one id names one thing",
            prev.address, prev.file
        ),
        format!(
            "edit it where it is declared, in `{}`, or write a change; or rename one",
            prev.file
        ),
    )
    .chain(chain)
    .related(
        Subject::parse(&prev.address).unwrap_or(Subject::World),
        At::Span(prev.header),
        format!("declared in `{}`", prev.file),
    )
}

/// A path made absolute and without `.` or `..`, by text: the file may not exist.
fn canonical(p: &Path) -> PathBuf {
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    };
    let mut out = PathBuf::new();
    for c in abs.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// A world file as a path from the parent's world folder, `/`-separated.
fn shown_path(dir: &Path, p: &Path) -> String {
    let dir = canonical(dir);
    let a: Vec<_> = dir.components().collect();
    let b: Vec<_> = p.components().collect();
    let same = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<String> = vec!["..".to_string(); a.len() - same];
    parts.extend(
        b[same..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
}

/// Follows the imports of the world at `p`: the loop, from the world it returns to, when one
/// comes back to a world in `stack`.
fn import_loop(p: &Path, stack: &mut Vec<PathBuf>, src: SrcId) -> Option<Vec<PathBuf>> {
    if let Some(at) = stack.iter().position(|q| q == p) {
        let mut l = stack[at..].to_vec();
        l.push(p.to_path_buf());
        return Some(l);
    }
    // A tree deeper than this is a loop by another path name, or a mistake.
    if stack.len() > 64 {
        return None;
    }
    let text = std::fs::read(p).ok().and_then(|b| decode(b, "").ok())?;
    let root = read::read(src, &text).ok()?;
    let dir = p.parent().map(Path::to_path_buf).unwrap_or_default();
    stack.push(p.to_path_buf());
    let mut found = None;
    if let Some(imports) = root.table("import") {
        for (_, e) in imports.iter() {
            if let Some(Value::Str(f)) = match &e.item.value {
                Value::Table(t) => t.value("file"),
                _ => None,
            } {
                if let Some(l) = import_loop(&canonical(&dir.join(&f.text)), stack, src) {
                    found = Some(l);
                    break;
                }
            }
        }
    }
    stack.pop();
    found
}
