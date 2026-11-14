//! The `via` command line.
//!
//! Responsibility: the commands a user, an agent and the conformance runner call (R20: the command
//! line is the interface every case runs through), and the build directory a build writes. Every
//! result reaches the user as text or as canonical JSON, with diagnostics from `via-diag`.
//!
//! Governed by `spec/build.md` (section 14 for the commands, sections 1 to 13 for the build
//! directory), with `spec/protocol.md` 6 and 7 for steps and trust and `spec/diagnostics.md` 2 and
//! 7 for `explain` and where records go. `--diagnostics FILE` is honoured on every command,
//! wherever it appears, because the conformance runner reads every record from that file.
//!
//! # Map
//!
//! | Module | Section |
//! |---|---|
//! | `build` | 13, 14: `via build` from reading to the run report |
//! | `out` | 2 to 9: the files of the `id` and the manifest's world keys |
//! | `layout` | 5: chunks |
//! | `changes` | 11.2: `changes.json` |
//! | `snap` | 11.2: the baseline kept in the store |
//! | `dir` | 13: replacing a directory whole, recovery, what `via` may delete |
//! | `check` | 14: `via check` |
//! | `steps` | `protocol.md` 2, 6, 7, 8.1: lint steps as plain commands, trust, the carried SDK |
//! | `cmds` | 14: `merge`, `explain`, `eval`, `vectors`, `trust`, `config program` |
//! | `kernel` | 14, `geometry.md` 7: `via kernel` |
//! | `pack` | 14, `assets.md` 8, 4.3: `via pack check`, `via pack list`, `via pack accept` |
//! | `query` | 14, `surfaces.md` 7: `via query` |

mod build;
mod changes;
mod check;
mod cmds;
mod commit;
mod dir;
mod filebase;
mod interrupt;
mod json;
mod kernel;
mod layout;
mod npy;
mod out;
mod pack;
mod query;
mod snap;
mod stepout;
mod steprun;
mod steps;
mod stepview;
mod stepwork;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use via_diag::nearest::nearest;
use via_diag::terminal::{self, Options};
use via_diag::{
    codes, document, exit_status, fail_on_code, Diag, Lint, NoWorld, Record, Site, Subject, Text,
};
use via_doc::ChangeRequest;

/// The engine version, from the workspace manifest, so the binary and `Cargo.toml` cannot disagree.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The longest `--step-deadline`, in seconds: one day (`build.md` 14).
const MAX_STEP_DEADLINE: u64 = 86_400;

const HELP: &str = "\
usage: via <command> [arguments] [--diagnostics FILE]

commands:
  build WORLD      build the world into its build directory
  merge WORLD      print the merged document as canonical JSON
  check DIR        check a build directory; writes nothing
  explain [CODE]   print a code's registry row; every code with none
  explain NODE WORLD   print a node's cause in the last build
  eval WORLD --field F [--window i0,j0,i1,j1] [--tile N] --out FILE
                   evaluate one scalar field into an array file
  vectors FILE     compute each vector of a vector file
  kernel OP --in FILE --out FILE
                   run one geometry operation on the shapes in a file
  pack check PACK  check a pack without a world; writes nothing
  pack list PACK [--taxon @PATH] [--tag TAG] [--build DIR]
                   print each asset's resolved facts as JSON Lines
  pack accept PACK write the pack's facts file
  query WORLD surfaces_at|sample|relief|meetings [--in FILE] --out FILE
                   answer one surface question over the whole world
  trust WORLD      list the world's steps and record trust
  step commit WORLD [--change C]... [--step ID]...
                   build, then write each generate step's result to gen/<id>/
  config program NAME PATH   record a named program's path
  help             print this text
  --version        print the engine version

flags of build:
  --change C  --out DIR  --lib DIR  --threads N  --store DIR  --against DIR
  --fail-on CODE  --rerun-step ID  --step-deadline SECONDS  --sdk DIR  --json  --info
";

/// A diagnostic text built at run time; a blank one would be a defect of ours, so it falls back
/// to a fixed text instead of panicking.
pub(crate) fn txt(s: String) -> Text {
    Text::new(s).unwrap_or_else(|_| via_diag::text!("(no text)"))
}

/// A `cli.usage` record at the 1-based argument `argument`.
pub(crate) fn usage(argument: u32, message: String, hint: String) -> Diag {
    Diag::new(
        codes::cli::USAGE,
        Subject::Cli,
        Site::Argument(argument),
        txt(message),
        txt(hint),
    )
}

const COMMANDS: [&str; 15] = [
    "--help",
    "--version",
    "build",
    "check",
    "config",
    "eval",
    "explain",
    "help",
    "kernel",
    "merge",
    "pack",
    "query",
    "step",
    "trust",
    "vectors",
];

/// Flags that take a value.
const VALUED: [&str; 20] = [
    "--build",
    "--tag",
    "--taxon",
    "--against",
    "--change",
    "--diagnostics",
    "--fail-on",
    "--field",
    "--in",
    "--lib",
    "--out",
    "--rerun-step",
    "--sdk",
    "--step",
    "--step-deadline",
    "--store",
    "--threads",
    "--tile",
    "--window",
    "--option",
];

/// Flags without a value.
const SWITCHES: [&str; 6] = [
    "--info",
    "--json",
    "--lowered",
    "--origins",
    "--stand-ins",
    "--yes",
];

/// The arguments, read: positional words and flags, each with its 1-based position.
#[derive(Default)]
struct Parsed {
    words: Vec<(u32, String)>,
    flags: Vec<(u32, String, Option<String>)>,
}

impl Parsed {
    fn values(&self, f: &str) -> Vec<(u32, String)> {
        self.flags
            .iter()
            .filter(|(_, n, _)| n == f)
            .filter_map(|(a, _, v)| v.clone().map(|v| (*a + 1, v)))
            .collect()
    }

    fn one(&self, f: &str) -> Option<(u32, String)> {
        self.values(f).pop()
    }

    fn has(&self, f: &str) -> Option<u32> {
        self.flags
            .iter()
            .find(|(_, n, _)| n == f)
            .map(|(a, _, _)| *a)
    }
}

fn read_args(raw: Vec<std::ffi::OsString>) -> (Parsed, Vec<Diag>) {
    let mut p = Parsed::default();
    let mut faults = Vec::new();
    let mut it = raw
        .into_iter()
        .enumerate()
        .map(|(i, a)| (u32::try_from(i + 1).unwrap_or(u32::MAX), a));
    while let Some((at, a)) = it.next() {
        let a = match a.into_string() {
            Ok(t) => t,
            Err(raw) => {
                faults.push(usage(
                    at,
                    format!(
                        "argument {at} is not valid UTF-8 (`{}`)",
                        raw.to_string_lossy()
                    ),
                    "write the argument in UTF-8".into(),
                ));
                continue;
            }
        };
        if a.starts_with("--") && a != "--help" && a != "--version" {
            if VALUED.contains(&a.as_str()) {
                match it.next().map(|(_, v)| v.into_string()) {
                    Some(Ok(v)) if !v.is_empty() => p.flags.push((at, a, Some(v))),
                    _ => faults.push(usage(
                        at,
                        format!("`{a}` needs a value"),
                        format!("write `{a} VALUE`"),
                    )),
                }
            } else if SWITCHES.contains(&a.as_str()) {
                p.flags.push((at, a, None));
            } else {
                let known = VALUED.iter().chain(SWITCHES.iter()).copied();
                let near = nearest(&a, known);
                let mut d = usage(
                    at,
                    format!("unknown flag `{a}`"),
                    near.hint("run `via help`"),
                );
                if let Some(n) = near.found() {
                    d = d.with("nearest", n);
                }
                faults.push(d);
            }
        } else {
            p.words.push((at, a));
        }
    }
    (p, faults)
}

/// Flags each command takes; any other is `cli.usage`.
fn allowed(command: &str) -> &'static [&'static str] {
    match command {
        "build" => &[
            "--against",
            "--change",
            "--diagnostics",
            "--fail-on",
            "--info",
            "--json",
            "--lib",
            "--out",
            "--rerun-step",
            "--sdk",
            "--stand-ins",
            "--step-deadline",
            "--store",
            "--threads",
        ],
        "merge" => &["--change", "--diagnostics", "--lib", "--origins", "--sdk"],
        "check" => &["--diagnostics", "--json", "--info"],
        "explain" => &["--diagnostics", "--json", "--out"],
        "eval" => &[
            "--change",
            "--diagnostics",
            "--field",
            "--json",
            "--lib",
            "--out",
            "--sdk",
            "--tile",
            "--window",
        ],
        "vectors" => &["--diagnostics"],
        "kernel" => &["--diagnostics", "--in", "--out"],
        "pack" => &["--build", "--diagnostics", "--tag", "--taxon"],
        "query" => &[
            "--change",
            "--diagnostics",
            "--in",
            "--lib",
            "--out",
            "--sdk",
            "--store",
        ],
        "step" => &[
            "--change",
            "--diagnostics",
            "--info",
            "--json",
            "--lib",
            "--out",
            "--sdk",
            "--step",
            "--step-deadline",
            "--store",
        ],
        "trust" => &[
            "--change",
            "--diagnostics",
            "--json",
            "--lib",
            "--sdk",
            "--yes",
        ],
        _ => &["--diagnostics"],
    }
}

fn changes_of(p: &Parsed) -> Vec<ChangeRequest> {
    p.values("--change")
        .into_iter()
        .map(|(at, c)| ChangeRequest::new(c, at))
        .collect()
}

fn libs_of(p: &Parsed) -> Vec<PathBuf> {
    p.values("--lib")
        .into_iter()
        .map(|(_, l)| PathBuf::from(l))
        .collect()
}

/// What a command gives: its records, lint list, exit status override, and build directory.
struct Done {
    records: Vec<Record>,
    lints: Vec<Lint>,
    status: Option<u8>,
    out: Option<PathBuf>,
    /// The diagnostics document, when the command made it itself (`via kernel`, whose records
    /// are those of its output file).
    document: Option<String>,
}

impl Done {
    fn of(records: Vec<Record>) -> Done {
        Done {
            records,
            lints: Vec::new(),
            status: None,
            out: None,
            document: None,
        }
    }
}

fn need_world(p: &Parsed, command: &str) -> Result<(u32, PathBuf), Diag> {
    p.words
        .get(1)
        .map(|(a, w)| (*a, PathBuf::from(w)))
        .ok_or_else(|| {
            usage(
                1,
                format!("`via {command}` needs a world file"),
                format!("write `via {command} WORLD`"),
            )
        })
}

#[allow(clippy::too_many_lines)]
fn dispatch(p: &Parsed) -> Result<Done, Vec<Diag>> {
    let Some((at, command)) = p.words.first().cloned() else {
        if p.has("--version").is_some() {
            println!("via {VERSION}");
            return Ok(Done::of(Vec::new()));
        }
        return Err(vec![usage(
            1,
            "no command given".into(),
            "run `via help`".into(),
        )]);
    };
    if !COMMANDS.contains(&command.as_str()) {
        let near = nearest(&command, COMMANDS);
        let mut d = usage(
            at,
            format!("unknown command `{command}`"),
            near.hint("run `via help`"),
        );
        if let Some(n) = near.found() {
            d = d.with("nearest", n);
        }
        return Err(vec![d]);
    }
    let ok = allowed(&command);
    let bad: Vec<Diag> = p
        .flags
        .iter()
        .filter(|(_, f, _)| !ok.contains(&f.as_str()))
        .map(|(a, f, _)| {
            if f == "--json" && matches!(command.as_str(), "merge" | "vectors" | "pack") {
                usage(*a, format!("`--json` is not taken by `via {command}`, whose standard output is its result"), "use `--diagnostics FILE`".into())
            } else {
                usage(*a, format!("`via {command}` takes no `{f}`"), "run `via help`".into())
            }
        })
        .collect();
    if !bad.is_empty() {
        return Err(bad);
    }
    let extra = |max: usize| -> Result<(), Vec<Diag>> {
        match p.words.get(max) {
            Some((a, w)) => Err(vec![usage(
                *a,
                format!("unexpected argument `{w}`"),
                "run `via help`".into(),
            )]),
            None => Ok(()),
        }
    };
    match command.as_str() {
        "--version" => {
            extra(1)?;
            println!("via {VERSION}");
            Ok(Done::of(Vec::new()))
        }
        "help" | "--help" => {
            extra(1)?;
            print!("{HELP}");
            Ok(Done::of(Vec::new()))
        }
        "build" => {
            if matches!(p.words.get(1), Some((_, w)) if w == "--help" || w == "-h") {
                extra(2)?;
                print!("{HELP}");
                return Ok(Done::of(Vec::new()));
            }
            extra(2)?;
            let (wa, world) = need_world(p, "build").map_err(|d| vec![d])?;
            let mut fail_on = Vec::new();
            let mut faults = Vec::new();
            for (a, c) in p.values("--fail-on") {
                match fail_on_code(&c, a) {
                    Ok(c) => fail_on.push(c),
                    Err(d) => faults.push(d),
                }
            }
            let threads = match p.one("--threads") {
                Some((a, t)) => match t.parse::<usize>() {
                    Ok(n) if n >= 1 => Some(n),
                    _ => {
                        faults.push(usage(
                            a,
                            format!("`--threads {t}` is not a whole number of at least 1"),
                            "write `--threads N`".into(),
                        ));
                        None
                    }
                },
                None => None,
            };
            let deadline = match p.one("--step-deadline") {
                // `build.md` 14: 1 to 86,400 seconds (one day); a longer wait is no deadline.
                Some((a, t)) => match t.parse::<u64>() {
                    Ok(n) if (1..=MAX_STEP_DEADLINE).contains(&n) => Some(n),
                    _ => {
                        faults.push(usage(
                            a,
                            format!(
                                "`--step-deadline {t}` is not a whole number of seconds from 1 to {MAX_STEP_DEADLINE}"
                            ),
                            "write `--step-deadline SECONDS`, at most 86400".into(),
                        ));
                        None
                    }
                },
                None => None,
            };
            if !faults.is_empty() {
                return Err(faults);
            }
            let o = build::run(&build::Args {
                world,
                world_arg: wa,
                changes: changes_of(p),
                out: p.one("--out").map(|(_, o)| PathBuf::from(o)),
                lib: libs_of(p),
                threads,
                store: p.one("--store").map(|(_, s)| PathBuf::from(s)),
                against: p.one("--against").map(|(_, s)| PathBuf::from(s)),
                fail_on,
                rerun: p
                    .values("--rerun-step")
                    .into_iter()
                    .map(|(_, s)| s)
                    .collect(),
                deadline,
                sdk: p.one("--sdk").map(|(_, s)| PathBuf::from(s)),
                diagnostics: p.one("--diagnostics").map(|(a, f)| (a, PathBuf::from(f))),
                stand_ins: p.has("--stand-ins").is_some(),
            });
            Ok(Done {
                records: o.records,
                lints: o.lints,
                status: None,
                out: o.out,
                document: None,
            })
        }
        "merge" => {
            extra(2)?;
            let (wa, world) = need_world(p, "merge").map_err(|d| vec![d])?;
            Ok(Done::of(cmds::merge(
                &world,
                wa,
                changes_of(p),
                libs_of(p),
                p.has("--origins").is_some(),
            )))
        }
        "check" => {
            extra(2)?;
            let Some((_, d)) = p.words.get(1) else {
                return Err(vec![usage(
                    1,
                    "`via check` needs a build directory".into(),
                    "write `via check DIR`".into(),
                )]);
            };
            let records = check::check(Path::new(d))
                .iter()
                .map(|d| d.emit(&NoWorld))
                .collect();
            Ok(Done::of(records))
        }
        "explain" => {
            extra(3)?;
            let code = p.words.get(1).map(|(a, c)| (c.as_str(), *a));
            let world = p.words.get(2).map(|(_, w)| PathBuf::from(w));
            let out = p.one("--out").map(|(_, o)| PathBuf::from(o));
            Ok(Done::of(cmds::explain(
                code,
                world.as_deref(),
                out.as_deref(),
                p.has("--json").is_some(),
            )))
        }
        "eval" => {
            extra(2)?;
            let (wa, world) = need_world(p, "eval").map_err(|d| vec![d])?;
            let Some((fa, field)) = p.one("--field") else {
                return Err(vec![usage(
                    1,
                    "`via eval` needs `--field F`".into(),
                    "write `--field F`".into(),
                )]);
            };
            let Some((_, out)) = p.one("--out") else {
                return Err(vec![usage(
                    1,
                    "`via eval` needs `--out FILE`".into(),
                    "write `--out FILE`".into(),
                )]);
            };
            let window = match p.one("--window") {
                Some((a, w)) => match cmds::parse_window(&w) {
                    Some(v) => Some((v, a)),
                    None => {
                        return Err(vec![usage(
                            a,
                            format!("`--window {w}` is not four whole numbers"),
                            "write `--window i0,j0,i1,j1`".into(),
                        )])
                    }
                },
                None => None,
            };
            let tile = match p.one("--tile") {
                Some((a, t)) => match t.parse::<u64>() {
                    Ok(n) if n >= 1 => Some(n),
                    _ => {
                        return Err(vec![usage(
                            a,
                            format!("`--tile {t}` is not a whole number of at least 1"),
                            "write `--tile N`".into(),
                        )])
                    }
                },
                None => None,
            };
            Ok(Done::of(cmds::eval(&cmds::EvalArgs {
                world,
                world_arg: wa,
                changes: changes_of(p),
                lib: libs_of(p),
                field: (field, fa),
                window,
                tile,
                out: PathBuf::from(out),
            })))
        }
        "vectors" => {
            extra(2)?;
            let Some((a, f)) = p.words.get(1) else {
                return Err(vec![usage(
                    1,
                    "`via vectors` needs a vector file".into(),
                    "write `via vectors FILE`".into(),
                )]);
            };
            Ok(Done::of(cmds::vectors(Path::new(f), *a)))
        }
        "step" => {
            extra(3)?;
            match p.words.get(1) {
                Some((_, w)) if w == "commit" => {}
                other => {
                    return Err(vec![usage(
                        other.map_or(1, |(a, _)| *a),
                        "`via step` takes `commit WORLD`".into(),
                        "write `via step commit WORLD`".into(),
                    )])
                }
            }
            let Some((wa, world)) = p.words.get(2).map(|(a, w)| (*a, PathBuf::from(w))) else {
                return Err(vec![usage(
                    2,
                    "`via step commit` needs a world file".into(),
                    "write `via step commit WORLD`".into(),
                )]);
            };
            let deadline = match p.one("--step-deadline") {
                Some((a, t)) => match t.parse::<u64>() {
                    Ok(n) if (1..=MAX_STEP_DEADLINE).contains(&n) => Some(n),
                    _ => {
                        return Err(vec![usage(
                            a,
                            format!(
                                "`--step-deadline {t}` is not a whole number of seconds from 1 to {MAX_STEP_DEADLINE}"
                            ),
                            "write `--step-deadline SECONDS`, at most 86400".into(),
                        )])
                    }
                },
                None => None,
            };
            let o = commit::run(
                &build::Args {
                    world,
                    world_arg: wa,
                    changes: changes_of(p),
                    out: p.one("--out").map(|(_, o)| PathBuf::from(o)),
                    lib: libs_of(p),
                    threads: None,
                    store: p.one("--store").map(|(_, s)| PathBuf::from(s)),
                    against: None,
                    fail_on: Vec::new(),
                    rerun: Vec::new(),
                    deadline,
                    sdk: p.one("--sdk").map(|(_, s)| PathBuf::from(s)),
                    diagnostics: p.one("--diagnostics").map(|(a, f)| (a, PathBuf::from(f))),
                    stand_ins: false,
                },
                &p.values("--step"),
            );
            Ok(Done {
                records: o.records,
                lints: o.lints,
                status: None,
                out: o.out,
                document: None,
            })
        }
        "trust" => {
            extra(2)?;
            let (wa, world) = need_world(p, "trust").map_err(|d| vec![d])?;
            let sdk = p.one("--sdk").map(|(_, s)| PathBuf::from(s));
            let (records, status) = cmds::trust(
                &world,
                wa,
                changes_of(p),
                libs_of(p),
                sdk.as_deref(),
                p.has("--yes").is_some(),
                p.has("--json").is_some(),
            );
            Ok(Done {
                records,
                lints: Vec::new(),
                status: Some(status),
                out: None,
                document: None,
            })
        }
        "kernel" => {
            extra(2)?;
            let Some((oa, op)) = p.words.get(1) else {
                return Err(vec![usage(
                    1,
                    "`via kernel` needs an operation".into(),
                    "write `via kernel OP --in FILE --out FILE`".into(),
                )]);
            };
            let (Some((ia, inp)), Some((outa, outp))) = (p.one("--in"), p.one("--out")) else {
                return Err(vec![usage(
                    *oa,
                    "`via kernel` needs `--in FILE` and `--out FILE`".into(),
                    "write `via kernel OP --in FILE --out FILE`".into(),
                )]);
            };
            let r = kernel::kernel(
                (*oa, op.as_str()),
                (ia, Path::new(&inp)),
                (outa, Path::new(&outp)),
            );
            Ok(Done {
                records: r.records,
                lints: Vec::new(),
                status: Some(r.status),
                out: None,
                document: r.document,
            })
        }
        "pack" => {
            extra(3)?;
            let (Some((sa, sub)), Some((pa, pk))) = (p.words.get(1), p.words.get(2)) else {
                return Err(vec![usage(
                    1,
                    "`via pack` takes `check PACK`, `list PACK` or `accept PACK`".into(),
                    "write `via pack check PACK`, `via pack list PACK` or `via pack accept PACK`"
                        .into(),
                )]);
            };
            match sub.as_str() {
                "check" | "accept" => {
                    if let Some((a, f, _)) = p
                        .flags
                        .iter()
                        .find(|(_, f, _)| f == "--taxon" || f == "--tag" || f == "--build")
                    {
                        return Err(vec![usage(
                            *a,
                            format!("`via pack {sub}` takes no `{f}`"),
                            "run `via pack list` to select".into(),
                        )]);
                    }
                    Ok(Done::of(if sub == "check" {
                        pack::check(Path::new(pk), *pa)
                    } else {
                        pack::accept(Path::new(pk), *pa)
                    }))
                }
                "list" => Ok(Done::of(pack::list(
                    Path::new(pk),
                    *pa,
                    p.one("--taxon"),
                    p.one("--tag"),
                    p.one("--build"),
                ))),
                other => {
                    let near = nearest(other, ["accept", "check", "list"]);
                    let mut d = usage(
                        *sa,
                        format!("`via pack {other}` is not a command"),
                        near.hint("write `check`, `list` or `accept`"),
                    );
                    if let Some(n) = near.found() {
                        d = d.with("nearest", n);
                    }
                    Err(vec![d])
                }
            }
        }
        "query" => {
            extra(3)?;
            let (wa, world) = need_world(p, "query").map_err(|d| vec![d])?;
            let Some((qa, q)) = p.words.get(2).cloned() else {
                return Err(vec![usage(
                    wa,
                    "`via query` needs a question".into(),
                    "write `via query WORLD surfaces_at|sample|relief|meetings --in FILE --out FILE`".into(),
                )]);
            };
            let Some((outa, out)) = p.one("--out") else {
                return Err(vec![usage(
                    qa,
                    "`via query` needs `--out FILE`".into(),
                    "write `--out FILE`".into(),
                )]);
            };
            Ok(Done::of(query::query(&query::QueryArgs {
                world,
                world_arg: wa,
                question: (q, qa),
                input: p.one("--in").map(|(a, f)| (PathBuf::from(f), a)),
                out: (PathBuf::from(out), outa),
                changes: changes_of(p),
                lib: libs_of(p),
            })))
        }
        "config" => {
            extra(4)?;
            match (p.words.get(1), p.words.get(2), p.words.get(3)) {
                (Some((_, w)), Some((_, name)), Some((a, path))) if w == "program" => {
                    Ok(Done::of(cmds::config_program(name, path, *a)))
                }
                _ => Err(vec![usage(
                    1,
                    "`via config` takes `program NAME PATH`".into(),
                    "write `via config program NAME PATH`".into(),
                )]),
            }
        }
        _ => Err(vec![usage(
            at,
            format!("unknown command `{command}`"),
            "run `via help`".into(),
        )]),
    }
}

fn main() -> ExitCode {
    interrupt::install();
    let (p, faults) = read_args(std::env::args_os().skip(1).collect());
    let diagnostics = p.one("--diagnostics");
    let json = p.has("--json").is_some()
        && !matches!(
            p.words.first().map(|w| w.1.as_str()),
            Some("trust" | "explain" | "merge" | "vectors" | "pack")
        );
    let info = p.has("--info").is_some();
    let done = if faults.is_empty() {
        dispatch(&p)
    } else {
        Err(faults)
    };
    let done = match done {
        Ok(d) => d,
        Err(faults) => Done::of(via_diag::finish(
            faults.iter().map(|d| d.emit(&NoWorld)).collect(),
            &via_diag::FailOn::none(),
        )),
    };
    let mut status = exit_status(&done.records).max(done.status.unwrap_or(0));
    if !done.records.is_empty() {
        let colour = std::env::var_os("NO_COLOR").is_none()
            && std::io::IsTerminal::is_terminal(&std::io::stderr());
        eprint!(
            "{}",
            terminal::render(&done.records, &done.lints, Options { info, colour })
        );
    }
    let doc = done
        .document
        .clone()
        .unwrap_or_else(|| document(&done.records, &done.lints, status));
    if json {
        print!("{doc}");
    }
    if let Some((at, file)) = diagnostics {
        // Never written into the build directory: `via build` refused it there (14).
        let inside = done
            .out
            .as_ref()
            .is_some_and(|o| build::lies_inside(Path::new(&file), o));
        if !inside {
            if let Err(e) = std::fs::write(&file, &doc) {
                let d = Diag::new(
                    codes::build::WRITE_FAILED,
                    Subject::Cli,
                    Site::Argument(at),
                    txt(format!(
                        "cannot write the diagnostics document `{file}`: {e}"
                    )),
                    via_diag::text!("free space, or fix the folder's permissions"),
                )
                .emit(&NoWorld);
                eprint!("{}", terminal::render(&[d], &[], Options::default()));
                status = status.max(1);
            }
        }
    }
    ExitCode::from(status)
}
