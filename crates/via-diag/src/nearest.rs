//! The nearest name (`spec/diagnostics.md` 1.4), which every chapter's "nearest name" means.
//!
//! Optimal-string-alignment distance (Damerau, 1964; the restricted form as in Boytsov, 2011)
//! over Unicode scalar values; accepted when at most `max(1, floor(length / 3))` of the written
//! name (the threshold is ours); ties to the bytewise-first name. Without one close enough, up to
//! five visible names, the nearest first, ties bytewise.

/// The answer for one written name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Nearest<'a> {
    /// The nearest name, close enough; it goes in `data.nearest`.
    Found(&'a str),
    /// None close enough: up to five visible names, the nearest first, ties bytewise, for the hint.
    None(Vec<&'a str>),
}

impl Nearest<'_> {
    /// A hint: "write `x`" when found; otherwise the listed names, or `otherwise` when there are
    /// none to list.
    pub fn hint(&self, otherwise: &str) -> String {
        match self {
            Nearest::Found(n) => format!("write `{n}`"),
            Nearest::None(list) if list.is_empty() => otherwise.to_string(),
            Nearest::None(list) => {
                let names: Vec<String> = list.iter().map(|n| format!("`{n}`")).collect();
                format!("names visible here include {}", names.join(", "))
            }
        }
    }

    /// The name found, if any.
    pub fn found(&self) -> Option<&str> {
        match self {
            Nearest::Found(n) => Some(n),
            Nearest::None(_) => None,
        }
    }
}

/// The nearest of `visible` to `written`.
pub fn nearest<'a>(written: &str, visible: impl IntoIterator<Item = &'a str>) -> Nearest<'a> {
    let mut names: Vec<&'a str> = visible.into_iter().collect();
    names.sort_unstable_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    names.dedup();
    let limit = (written.chars().count() / 3).max(1);
    let mut best: Option<(usize, &'a str)> = None;
    for n in &names {
        let d = osa_distance(written, n);
        // Names are in bytewise order, so a strict `<` keeps the bytewise-first on a tie.
        if d <= limit && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, n));
        }
    }
    match best {
        Some((_, n)) => Nearest::Found(n),
        // The nearest five, ties bytewise (the sort is stable): `house_tall` lists `house_low`
        // before `barn` (M7 close, a fresh agent's hint).
        None => {
            names.sort_by_key(|n| osa_distance(written, n));
            Nearest::None(names.into_iter().take(5).collect())
        }
    }
}

/// Optimal-string-alignment distance: insertions, deletions, substitutions and transpositions
/// of adjacent characters, no substring edited twice.
pub fn osa_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut v = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = v;
        }
    }
    d[n][m]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance() {
        assert_eq!(osa_distance("ca", "abc"), 3); // OSA, not unrestricted Damerau (2)
        assert_eq!(osa_distance("hieght", "height"), 1);
        assert_eq!(osa_distance("", "abc"), 3);
        assert_eq!(osa_distance("xx", "x"), 1);
    }

    #[test]
    fn finds_within_threshold() {
        assert_eq!(nearest("xx", ["x", "y", "height"]), Nearest::Found("x"));
        assert_eq!(
            nearest(
                "formula.unknwn_name",
                ["formula.unknown_name", "formula.unmarked"]
            ),
            Nearest::Found("formula.unknown_name")
        );
        // Ties go to the bytewise-first name.
        assert_eq!(nearest("ab", ["bb", "aa"]), Nearest::Found("aa"));
    }

    #[test]
    fn lists_five_when_none_is_close() {
        let r = nearest("zzzzzz", ["f", "e", "d", "c", "b", "a"]);
        assert_eq!(r, Nearest::None(vec!["a", "b", "c", "d", "e"]));
        assert_eq!(
            r.hint("-"),
            "names visible here include `a`, `b`, `c`, `d`, `e`"
        );
    }

    #[test]
    fn lists_the_nearest_first() {
        let names = [
            "barn",
            "box_small",
            "crate",
            "fence_post",
            "hay",
            "house_low",
            "house_wide",
        ];
        let r = nearest("house_tall", names);
        assert_eq!(
            r,
            Nearest::None(vec![
                "house_low",
                "house_wide",
                "box_small",
                "fence_post",
                "hay"
            ])
        );
    }
}
