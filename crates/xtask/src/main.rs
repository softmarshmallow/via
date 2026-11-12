//! `cargo run -p xtask -- check [--only NAME[,NAME...]] [--milestone N] [--no-local-patterns]`
//!
//! Exit 0 when every check passed or was skipped, 1 when one failed, 2 on a usage error.

use std::process::ExitCode;

const USAGE: &str =
    "usage: cargo run -p xtask -- check [--only NAME[,NAME...]] [--milestone N] [--no-local-patterns]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut opts = xtask::Options::default();
    let mut it = args.iter();
    if it.next().map(String::as_str) != Some("check") {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    while let Some(a) = it.next() {
        match a.as_str() {
            "--only" => match it.next() {
                Some(v) => opts.only.extend(v.split(',').map(str::to_string)),
                None => {
                    eprintln!("--only needs a check name\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            "--milestone" => match it.next().and_then(|v| v.parse::<u32>().ok()) {
                Some(m) if m <= 7 => opts.milestone = m,
                _ => {
                    eprintln!("--milestone needs a number 0 to 7\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            "--no-local-patterns" => opts.no_local_patterns = true,
            _ => {
                eprintln!("unknown argument `{a}`\n{USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    let names: Vec<&str> = xtask::CHECKS.iter().map(|(n, _)| *n).collect();
    if let Some(bad) = opts.only.iter().find(|o| !names.contains(&o.as_str())) {
        eprintln!("unknown check `{bad}`; checks are {}", names.join(", "));
        return ExitCode::from(2);
    }
    let root = xtask::repo_root();
    let mut stdout = std::io::stdout();
    match xtask::run(&root, &opts, &mut stdout) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("cannot write the report: {e}");
            ExitCode::from(2)
        }
    }
}
