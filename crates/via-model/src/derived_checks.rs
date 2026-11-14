//! Simulation under authority (`derived.md` 5, R14): which features are stated facts, which facts
//! a generate step read, the lint key `checks` and the lint request's `checks` member, and
//! `authority.unchecked`.
//!
//! These are pure functions over what the build already knows (each step's `when` and `checks`,
//! each feature's tier and whether it was written in text, each step's traced reads and what it
//! returned), so the step runner and the graph call them where they hold those facts.

use crate::decl::Tier;
use crate::text::{at_key, diag};
use std::collections::{BTreeMap, BTreeSet};
use via_diag::nearest::osa_distance;
use via_diag::{codes, Diag, KeyPath, Site, Subject};

/// A step as `checks` sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepInfo {
    pub id: String,
    /// `generate`, `lint` or `export`.
    pub when: String,
    /// The `checks` key, when written.
    pub checks: Option<Vec<String>>,
}

/// A stated fact (`derived.md` 5): a feature written in text (a world, a layer or a change) whose
/// tier is `shaped` or `anchored`. Nothing a step returns states anything.
pub fn is_stated_fact(tier: Tier, written_in_text: bool) -> bool {
    written_in_text && matches!(tier, Tier::Shaped | Tier::Anchored)
}

/// What a generate step read, for its facts.
#[derive(Clone, Debug)]
pub enum Reads<'a> {
    /// A worker: the names in the answers of its traced `build.features` reads, every item's
    /// under `for_each`.
    Worker(&'a [Vec<String>]),
    /// A plain command: it read every stated fact in its stratum.
    Command,
}

/// The stated facts a step read (`derived.md` 5): of `stated` (the stated facts in the step's
/// stratum), those a worker's traced `build.features` answers name, or every one for a plain
/// command. Sorted.
pub fn facts_read(stated: &BTreeSet<String>, reads: &Reads) -> BTreeSet<String> {
    match reads {
        Reads::Command => stated.clone(),
        Reads::Worker(answers) => answers
            .iter()
            .flatten()
            .filter(|n| stated.contains(*n))
            .cloned()
            .collect(),
    }
}

/// The `checks` key on every step: `derive.checks_when` on a step whose `when` is not `lint`, at its
/// `checks` key; `derive.checks_target` for an id that is no generate step of this world, at that
/// element, with the nearest generate step id.
pub fn check_keys(steps: &[StepInfo]) -> Vec<Diag> {
    let generate: Vec<&str> = steps
        .iter()
        .filter(|s| s.when == "generate")
        .map(|s| s.id.as_str())
        .collect();
    let mut out = Vec::new();
    for s in steps {
        let Some(checks) = &s.checks else { continue };
        if s.when != "lint" {
            out.push(at_key(
                codes::derive::CHECKS_WHEN,
                "step",
                &s.id,
                "checks",
                &format!(
                    "`checks` is written on a `{}` step; only a lint step checks",
                    s.when
                ),
                "move `checks` to a lint step",
            ));
            continue;
        }
        for (k, id) in checks.iter().enumerate() {
            if generate.contains(&id.as_str()) {
                continue;
            }
            let site = Site::Key(KeyPath::key("checks").index(k as u32));
            // The nearest generate step, by edit distance, ties bytewise; with no limit, since
            // any generate step is a better target than a step that cannot be checked.
            let near = generate.iter().copied().min_by(|a, b| {
                osa_distance(id, a)
                    .cmp(&osa_distance(id, b))
                    .then_with(|| a.as_bytes().cmp(b.as_bytes()))
            });
            let (message, hint) = match near {
                Some(n) => (
                    format!("`checks` names `{id}`, which is not a generate step of this world"),
                    format!("the nearest generate step is `{n}`"),
                ),
                None => (
                    format!("`checks` names `{id}`, which is not a generate step of this world"),
                    "name a step whose `when` is `generate`".to_string(),
                ),
            };
            let d = diag(
                codes::derive::CHECKS_TARGET,
                Subject::entity("step", &s.id),
                site,
                &message,
                &hint,
            );
            out.push(match near {
                Some(n) => d.nearest_name(n, Some((Subject::entity("step", n), Site::Header))),
                None => d,
            });
        }
    }
    out
}

/// One entry of a lint request's `checks` member: a checked step and the sorted names of the
/// stated facts it read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    pub step: String,
    pub facts: Vec<String>,
}

/// The lint request's `checks` member (`derived.md` 5, `protocol.md` 2): per named step, sorted by
/// id, the facts it read. `facts` holds each generate step's [`facts_read`]; a step with no entry
/// read none. A name repeated in `checks` is one entry.
pub fn checks_member(lint: &StepInfo, facts: &BTreeMap<String, BTreeSet<String>>) -> Vec<Checked> {
    let ids: BTreeSet<&String> = lint.checks.iter().flatten().collect();
    ids.into_iter()
        .map(|id| Checked {
            step: id.clone(),
            facts: facts
                .get(id)
                .map(|f| f.iter().cloned().collect())
                .unwrap_or_default(),
        })
        .collect()
}

/// `authority.unchecked`: a generate step that read a stated fact and returned rows or features,
/// named by no lint step's `checks`, at its `when` line. `facts` is each step's [`facts_read`];
/// `returned` the steps that returned rows or features.
pub fn unchecked(
    steps: &[StepInfo],
    facts: &BTreeMap<String, BTreeSet<String>>,
    returned: &BTreeSet<String>,
) -> Vec<Diag> {
    let checked: BTreeSet<&str> = steps
        .iter()
        .filter(|s| s.when == "lint")
        .flat_map(|s| s.checks.iter().flatten())
        .map(String::as_str)
        .collect();
    steps
        .iter()
        .filter(|s| s.when == "generate")
        .filter(|s| returned.contains(&s.id))
        .filter(|s| facts.get(&s.id).is_some_and(|f| !f.is_empty()))
        .filter(|s| !checked.contains(s.id.as_str()))
        .map(|s| {
            let read: Vec<&str> = facts
                .get(&s.id)
                .map(|f| f.iter().map(String::as_str).collect())
                .unwrap_or_default();
            at_key(
                codes::authority::UNCHECKED,
                "step",
                &s.id,
                "when",
                &format!(
                    "step `{}` read the stated facts {} and returned results, but no lint step checks it",
                    s.id,
                    read.iter()
                        .map(|n| format!("`{n}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                &format!("add a lint step with `checks = [\"{}\"]`", s.id),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use via_diag::DiagCode;

    fn step(id: &str, when: &str, checks: Option<&[&str]>) -> StepInfo {
        StepInfo {
            id: id.into(),
            when: when.into(),
            checks: checks.map(|c| c.iter().map(|s| s.to_string()).collect()),
        }
    }

    fn code(d: &Diag) -> DiagCode {
        d.code().clone()
    }

    #[test]
    fn stated_facts_are_text_and_firm() {
        assert!(is_stated_fact(Tier::Anchored, true));
        assert!(is_stated_fact(Tier::Shaped, true));
        assert!(!is_stated_fact(Tier::Derived, true));
        assert!(!is_stated_fact(Tier::Anchored, false));
    }

    #[test]
    fn d16_a_step_that_read_a_fact_unchecked() {
        let steps = [step("terrain", "generate", None)];
        let stated: BTreeSet<String> = ["haven".to_string()].into();
        let answers = vec![vec!["haven".to_string(), "meadow".to_string()]];
        let read = facts_read(&stated, &Reads::Worker(&answers));
        let facts: BTreeMap<String, BTreeSet<String>> = [("terrain".to_string(), read)].into();
        let returned: BTreeSet<String> = ["terrain".to_string()].into();
        let d = unchecked(&steps, &facts, &returned);
        assert_eq!(d.len(), 1);
        assert_eq!(code(&d[0]), DiagCode::Core(codes::authority::UNCHECKED));
        assert_eq!(d[0].site(), &Site::Key(KeyPath::key("when")));
        assert!(d[0].undeclared_data().is_empty());
        // Nothing returned: nothing to check.
        assert!(unchecked(&steps, &facts, &BTreeSet::new()).is_empty());
    }

    #[test]
    fn d17_a_lint_with_checks_clears_it_and_gets_the_member() {
        let steps = [
            step("terrain", "generate", None),
            step("facts", "lint", Some(&["terrain"])),
        ];
        let stated: BTreeSet<String> = ["haven".to_string(), "spring".to_string()].into();
        let facts: BTreeMap<String, BTreeSet<String>> =
            [("terrain".to_string(), facts_read(&stated, &Reads::Command))].into();
        let returned: BTreeSet<String> = ["terrain".to_string()].into();
        assert!(unchecked(&steps, &facts, &returned).is_empty());
        assert!(check_keys(&steps).is_empty());
        assert_eq!(
            checks_member(&steps[1], &facts),
            vec![Checked {
                step: "terrain".into(),
                facts: vec!["haven".into(), "spring".into()],
            }]
        );
    }

    #[test]
    fn d18_checks_in_the_wrong_place() {
        let steps = [
            step("terrain", "generate", Some(&["terrain"])),
            step("tidy", "lint", None),
            step("facts", "lint", Some(&["tidy", "terain"])),
        ];
        let d = check_keys(&steps);
        assert_eq!(d.len(), 3);
        assert_eq!(code(&d[0]), DiagCode::Core(codes::derive::CHECKS_WHEN));
        assert_eq!(code(&d[1]), DiagCode::Core(codes::derive::CHECKS_TARGET));
        assert_eq!(code(&d[2]), DiagCode::Core(codes::derive::CHECKS_TARGET));
        assert_eq!(
            d[2].data().get("nearest"),
            Some(&via_diag::Value::text("terrain"))
        );
        assert_eq!(d[2].site(), &Site::Key(KeyPath::key("checks").index(1)));
        // Every fact a record carries is one the registry declares.
        assert!(d.iter().all(|x| x.undeclared_data().is_empty()));
    }
}
