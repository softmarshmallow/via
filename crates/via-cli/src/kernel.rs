//! `via kernel OP --in FILE --out FILE` (`geometry.md` 7): one geometry operation on the shapes in a
//! file. The work is `via_geom::kernel::run`; this module moves bytes, names the argument a usage
//! fault is about, writes the reproduction folder of 7.4, and gives the diagnostics document the
//! records the output file holds, so `--diagnostics` and the output agree byte for byte.

use std::path::Path;

use via_diag::nearest::nearest;
use via_diag::{codes, Diag, NoWorld, Position, Record, Site, Subject};

use crate::json::{parse, J};
use crate::{txt, usage};

/// What the command gives `main`: its records for the terminal, the diagnostics document when
/// the kernel wrote one, and the exit status.
pub struct Ran {
    pub records: Vec<Record>,
    pub document: Option<String>,
    pub status: u8,
}

fn fail(d: Diag) -> Ran {
    Ran {
        records: vec![d.emit(&NoWorld)],
        document: None,
        status: 2,
    }
}

/// The records of the output's `diagnostics` member, rebuilt for the terminal: code, message,
/// hint, position and source. The document itself is the output's own list (`document_of`).
fn terminal_records(list: &[J], in_arg: u32) -> Vec<Record> {
    let mut out = Vec::new();
    for r in list {
        let Some(code) = r
            .get("code")
            .and_then(J::as_str)
            .and_then(via_diag::registry::lookup)
        else {
            continue;
        };
        let s = |k: &str| r.get(k).and_then(J::as_str).unwrap_or("").to_string();
        let mut d = Diag::new(
            code,
            Subject::Cli,
            Site::Argument(in_arg),
            txt(s("message")),
            txt(s("hint")),
        );
        if let Some(p) = r.get("position").and_then(J::as_arr) {
            if let (Some(x), Some(y)) =
                (p.first().and_then(J::as_i64), p.get(1).and_then(J::as_i64))
            {
                d = d.at(Position::plan(x, y));
            }
        }
        if let Some(src) = r
            .get("data")
            .and_then(|d| d.get("source"))
            .and_then(J::as_str)
        {
            d = d.with("source", via_diag::Value::text(src));
        }
        out.push(d.emit(&NoWorld));
    }
    out
}

/// The diagnostics document (`diagnostics.md` 7) holding exactly the output's records.
fn document_of(list: &[J], status: u8) -> String {
    let mut counts = [0i64; 3];
    for r in list {
        match r.get("severity").and_then(J::as_str) {
            Some("error") => counts[0] += 1,
            Some("info") => counts[1] += 1,
            Some("warning") => counts[2] += 1,
            _ => {}
        }
    }
    let empty = via_diag::document(&[], &[], status);
    let Ok(J::Obj(mut o)) = parse(&empty) else {
        return empty;
    };
    let mut c = J::obj();
    c.insert("error".into(), J::int(counts[0]));
    c.insert("failed_on".into(), J::int(0));
    c.insert("info".into(), J::int(counts[1]));
    c.insert("warning".into(), J::int(counts[2]));
    o.insert("counts".into(), J::Obj(c));
    o.insert("diagnostics".into(), J::Arr(list.to_vec()));
    let mut s = J::Obj(o).canonical();
    s.push('\n');
    s
}

/// Runs the command. `op` and `inp` carry their 1-based argument positions.
pub fn kernel(op: (u32, &str), inp: (u32, &Path), outp: (u32, &Path)) -> Ran {
    let (op_arg, op) = op;
    let (in_arg, inp) = inp;
    let (out_arg, outp) = outp;
    if !via_geom::kernel::OPS.contains(&op) {
        let near = nearest(op, via_geom::kernel::OPS.iter().copied());
        let mut d = usage(
            op_arg,
            format!("`{op}` is not a kernel operation"),
            near.hint("see geometry.md 7.2"),
        );
        if let Some(n) = near.found() {
            d = d.with("nearest", n);
        }
        return fail(d);
    }
    let Ok(text) = std::fs::read_to_string(inp) else {
        return fail(usage(
            in_arg,
            format!("cannot read `{}`", inp.display()),
            "give a readable `via-kernel-1` input file".into(),
        ));
    };
    let o = via_geom::kernel::run(op, &text, in_arg);
    if let Some(u) = &o.usage {
        let (at, what) = if u == "OP" {
            (op_arg, format!("`{op}` is not a kernel operation"))
        } else if u == "--in" {
            (
                in_arg,
                format!("`{}` is not one JSON object", inp.display()),
            )
        } else {
            (
                in_arg,
                format!("the input's member `{u}` is missing or not of its form"),
            )
        };
        return fail(usage(at, what, "see geometry.md 7.1 and 7.2".into()));
    }
    let Some(output) = &o.output else {
        return fail(usage(
            in_arg,
            "the kernel gave no output".into(),
            "report it: this is a defect of the engine".into(),
        ));
    };
    if let Err(e) = std::fs::write(outp, output) {
        let d = Diag::new(
            codes::build::WRITE_FAILED,
            Subject::Cli,
            Site::Argument(out_arg),
            txt(format!("cannot write `{}`: {e}", outp.display())),
            via_diag::text!("free space, or fix the folder's permissions"),
        );
        return Ran {
            records: vec![d.emit(&NoWorld)],
            document: None,
            status: 1,
        };
    }
    if let Some((digest, input)) = &o.repro {
        let mut dir = outp.as_os_str().to_owned();
        dir.push(".repro");
        let dir = Path::new(&dir).join(digest);
        if std::fs::create_dir_all(&dir).is_ok() {
            let _ = std::fs::write(dir.join(format!("{op}.json")), input);
        }
    }
    let list: Vec<J> = parse(output)
        .ok()
        .and_then(|j| j.get("diagnostics").and_then(J::as_arr).map(<[J]>::to_vec))
        .unwrap_or_default();
    Ran {
        records: terminal_records(&list, in_arg),
        document: Some(document_of(&list, o.status)),
        status: o.status,
    }
}
