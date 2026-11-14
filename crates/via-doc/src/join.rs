//! What a step returns, joined to its layer (`protocol.md` 5, "Its layer"; `documents.md` 5).
//!
//! A generate step's document is read by the same reader as authored text, so every fault in it
//! names a line of `step:<id>/document.toml`. Each table it returns is declared in the step's
//! layer under its produced name `<step>#<key>` (`identity.md` 2.4): a field, a feature, an
//! instance, a rule; a row on a field of the stratum keeps the field's name and is named
//! `<step>#<row>` under it. Its chain carries the layer and its owner, so the layer's owner
//! answers for what its steps make (challenge 21).
//!
//! Names inside the document resolve first to its own keys (the formula checker's scope, set by
//! the model from the produced name's prefix), then to the stratum.

use crate::check::{Checker, Rank};
use crate::report::{At, Fault};
use crate::source::{Role, Source};
use crate::tree::{Entry, Form, Link, Table, Value};
use crate::world::World;
use via_diag::Through;
use via_diag::{codes, Record, Subject};

/// One generate step's checked document, to join.
#[derive(Clone, Debug)]
pub struct Returned {
    /// The step's id.
    pub step: String,
    /// The layer the step is declared in.
    pub layer: String,
    /// The document's text.
    pub text: String,
}

/// The file a returned document is located in (`documents.md` 2.1).
pub fn document_file(step: &str) -> String {
    format!("step:{step}/document.toml")
}

/// The tables a returned document may hold (`protocol.md` 5); the step's checks refuse the rest
/// before the join, which skips anything else.
const JOINED: [&str; 6] = ["field", "feature", "instance", "set", "run", "scatter"];

impl World {
    /// The world with these returned documents joined to their layers, and the faults found in
    /// them. A document with an error joins nothing (`protocol.md` 5: nothing from it is applied).
    pub fn join(&self, returned: &[Returned]) -> (World, Vec<Record>) {
        let mut w = self.clone();
        let mut records = Vec::new();
        for r in returned {
            let mut faults: Vec<Fault> = Vec::new();
            let owner = w.ranks.get(&r.layer).and_then(|k| k.owner.clone());
            let src = w.sources.add(Source::new(
                document_file(&r.step),
                Role::Layer {
                    id: r.layer.clone(),
                    owner: owner.clone(),
                },
                r.text.clone(),
            ));
            let mut root = match crate::read::read(src, &r.text) {
                Ok(t) => t,
                Err(fs) => {
                    records.extend(fs.iter().map(|f| f.emit(&w.sources, &w.world_file)));
                    continue;
                }
            };
            let layer = r.layer.clone();
            set_chain(&mut root, &|s| {
                vec![Link {
                    through: Through::Layer,
                    name: layer.clone(),
                    span: s,
                    owner: owner.clone(),
                }]
            });
            let mut copy = w.base.clone();
            for (top, e) in root.entries {
                let Value::Table(t) = e.item.value else {
                    continue;
                };
                if !JOINED.contains(&top.as_str()) {
                    continue;
                }
                for (key, ee) in t.entries {
                    let produced = format!("{}#{key}", r.step);
                    if top == "field"
                        && copy
                            .root
                            .table("field")
                            .and_then(|f| f.table(&key))
                            .is_some()
                    {
                        // Rows on a field of the stratum: each row under its produced name.
                        let Value::Table(ft) = ee.item.value else {
                            continue;
                        };
                        let Some(rows) = ft.get("then").and_then(|t| match &t.item.value {
                            Value::Table(rt) => Some(rt.clone()),
                            _ => None,
                        }) else {
                            continue;
                        };
                        let then_span = ft.get("then").map_or(e.key, |t| t.key);
                        let Some(field) =
                            copy.root.table_mut("field").and_then(|f| f.table_mut(&key))
                        else {
                            continue;
                        };
                        if field.get("then").is_none() {
                            field.insert(
                                "then".into(),
                                Entry {
                                    key: then_span,
                                    item: crate::tree::Item {
                                        span: then_span,
                                        value: Value::Table(Table::new(then_span, Form::Header)),
                                    },
                                    chain: Vec::new(),
                                },
                            );
                        }
                        if let Some(then) = field.table_mut("then") {
                            for (row, re) in rows.entries {
                                let name = format!("{}#{row}", r.step);
                                if then.get(&name).is_some() {
                                    faults.push(Fault::new(
                                        codes::identity::DUPLICATE_ID,
                                        Subject::entity("field", &key),
                                        At::Span(re.key),
                                        format!("the row `{name}` is returned twice"),
                                        "return each row once",
                                    ));
                                    continue;
                                }
                                then.insert(name, re);
                            }
                        }
                        continue;
                    }
                    if copy.homes.contains_key(&(top.clone(), produced.clone())) {
                        faults.push(Fault::new(
                            codes::identity::DUPLICATE_ID,
                            Subject::entity(top.as_str(), &produced),
                            At::Span(ee.key),
                            format!("`{top}.{produced}` is returned twice"),
                            "return each key once",
                        ));
                        continue;
                    }
                    if crate::load::top_mut(&mut copy.root, &top, e.key).is_some() {
                        if let Some(tt) = copy.root.table_mut(&top) {
                            tt.insert(produced.clone(), ee);
                        }
                    }
                    copy.homes.insert((top.clone(), produced), r.layer.clone());
                }
            }
            // The returned tables are checked by the same rules as authored text; only faults in
            // this document are its own.
            for f in Checker::new(&copy.root).run() {
                // A produced name is no id by the text's grammar, and needs none (`identity.md`
                // 2.4): its key was an id, checked before the step's name was put in front.
                let produced_id = f.code == codes::identity::BAD_ID && f.message.contains('#');
                if matches!(f.at, At::Span(sp) if sp.src == src) && !produced_id {
                    faults.push(f);
                }
            }
            let error = faults
                .iter()
                .any(|f| f.code.severity() == via_diag::Severity::Error);
            records.extend(faults.iter().map(|f| f.emit(&w.sources, &w.world_file)));
            if !error {
                w.base = copy;
            }
        }
        (w, records)
    }

    /// The step a produced name belongs to: the part before `#` of an entity id (`identity.md`
    /// 2.4), or of a row id under a field.
    pub fn producer(id: &str) -> Option<&str> {
        id.split_once('#').map(|(s, _)| s).filter(|s| !s.is_empty())
    }

    /// The layers of this world with their ranks, for a caller ordering generate steps.
    pub fn ranks(&self) -> Vec<(String, i64)> {
        self.ranks
            .iter()
            .map(|(k, r): (&String, &Rank)| (k.clone(), r.rank))
            .collect()
    }
}

fn set_chain(t: &mut Table, chain: &dyn Fn(crate::source::Span) -> Vec<Link>) {
    for e in t.entries.values_mut() {
        e.chain = chain(e.key);
        if let Value::Table(sub) = &mut e.item.value {
            set_chain(sub, chain);
        }
    }
}
