//! `kernel_cmd OP --in FILE --out FILE`: the `via kernel` command of `spec/geometry.md` 7, until
//! `via-cli` wires it. Moves bytes and sets the exit status; the work is `via_geom::kernel::run`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = |what: &str| {
        eprintln!("error[cli.usage]: {what}\n  usage: kernel OP --in FILE --out FILE");
        ExitCode::from(2)
    };
    if args.len() != 5 || args[1] != "--in" || args[3] != "--out" {
        return usage("expected OP --in FILE --out FILE");
    }
    let (op, inp, outp) = (&args[0], &args[2], &args[4]);
    let Ok(text) = std::fs::read_to_string(inp) else {
        return usage(&format!("cannot read `{inp}`"));
    };
    let o = via_geom::kernel::run(op, &text, 3);
    if let Some(u) = &o.usage {
        return usage(&format!("`{u}`"));
    }
    if let Some(out) = &o.output {
        if std::fs::write(outp, out).is_err() {
            return usage(&format!("cannot write `{outp}`"));
        }
    }
    if let Some((digest, input)) = &o.repro {
        let dir = format!("{outp}.repro/{digest}");
        if std::fs::create_dir_all(&dir).is_ok() {
            let _ = std::fs::write(format!("{dir}/{op}.json"), input);
        }
    }
    ExitCode::from(o.status)
}
