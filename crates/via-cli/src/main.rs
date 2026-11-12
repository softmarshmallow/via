//! The `via` command line.
//!
//! Responsibility: the commands a user, an agent and the conformance runner call (R20: the command
//! line is the interface every case runs through), and the build directory a build writes. Every
//! result reaches the user as text or as canonical JSON, with diagnostics from `via-diag`.
//!
//! Governed by `spec/build.md` (section 14 for the commands, sections 1 to 13 for the build
//! directory). At M0 the skeleton answers `via --version` and `via help` only; any other argument
//! is a usage error with exit status 2, as `spec/diagnostics.md` gives `cli.usage`.
//! `--diagnostics FILE` is honoured on every command, wherever it appears (`build.md` 14), because
//! the conformance runner reads every record from that file, exit 0 and 2 included.

use std::process::ExitCode;

use via_diag::nearest::nearest;
use via_diag::terminal::{self, Options};
use via_diag::{
    codes, document, exit_status, finish, Diag, FailOn, NoWorld, Record, Site, Subject,
};
use via_diag::{text, Text};

/// The engine version, from the workspace manifest, so the binary and `Cargo.toml` cannot disagree.
const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
usage: via <command> [--diagnostics FILE]

commands:
  help         print this text
  --version    print the engine version

flags:
  --diagnostics FILE   also write the diagnostics document to FILE
";

/// The commands of M0, for the nearest-name hint of an unknown one.
const COMMANDS: [&str; 3] = ["--help", "--version", "help"];

/// A diagnostic text built at run time; a blank one would be a defect of ours, so it falls back
/// to a fixed text instead of panicking.
fn txt(s: String) -> Text {
    Text::new(s).unwrap_or_else(|_| text!("(no text)"))
}

/// A `cli.usage` record at the 1-based argument `argument`.
fn usage(argument: usize, message: String, hint: String) -> Diag {
    let argument = u32::try_from(argument).unwrap_or(u32::MAX);
    Diag::new(
        codes::cli::USAGE,
        Subject::Cli,
        Site::Argument(argument),
        txt(message),
        txt(hint),
    )
}

/// What the command line asks for, once `--diagnostics FILE` is taken out.
enum Command {
    Version,
    Help,
}

/// An argument and its 1-based position, so a fault can name where it is.
type Arg = (usize, String);

/// Reads the arguments: the command, the `--diagnostics` file if any, or the usage records.
fn parse(raw: Vec<std::ffi::OsString>) -> (Result<Command, Vec<Diag>>, Option<Arg>) {
    let mut faults = Vec::new();
    let mut words: Vec<Arg> = Vec::new();
    let mut diagnostics: Option<Arg> = None;
    let mut iter = raw.into_iter().enumerate().map(|(i, a)| (i + 1, a));
    while let Some((position, arg)) = iter.next() {
        // `std::env::args` panics on an argument that is not valid UTF-8, which would abort the
        // process instead of reporting a usage error; such an argument is `cli.usage`.
        let arg = match arg.into_string() {
            Ok(text) => text,
            Err(raw) => {
                faults.push(usage(
                    position,
                    format!(
                        "argument {position} is not valid UTF-8 (`{}`)",
                        raw.to_string_lossy()
                    ),
                    "write the argument in UTF-8".to_string(),
                ));
                continue;
            }
        };
        if arg == "--diagnostics" {
            match iter.next() {
                Some((at, file)) => match file.into_string() {
                    Ok(f) if !f.is_empty() => diagnostics = Some((at, f)),
                    _ => faults.push(usage(
                        position + 1,
                        "`--diagnostics` needs a file path in UTF-8".to_string(),
                        "write `--diagnostics FILE`".to_string(),
                    )),
                },
                None => faults.push(usage(
                    position,
                    "`--diagnostics` needs a file".to_string(),
                    "write `--diagnostics FILE`".to_string(),
                )),
            }
        } else {
            words.push((position, arg));
        }
    }
    if !faults.is_empty() {
        return (Err(faults), diagnostics);
    }
    let command = match words.as_slice() {
        [(_, w)] if w == "--version" => Ok(Command::Version),
        [(_, w)] if w == "help" || w == "--help" => Ok(Command::Help),
        [] => Err(vec![usage(
            1,
            "no command given".to_string(),
            "run `via help`".to_string(),
        )]),
        [(position, w), ..] if !COMMANDS.contains(&w.as_str()) => {
            let near = nearest(w, COMMANDS);
            let hint = near.hint("run `via help`");
            let mut d = usage(*position, format!("unknown command `{w}`"), hint);
            if let Some(n) = near.found() {
                d = d.with("nearest", n);
            }
            Err(vec![d])
        }
        [.., (position, w)] => Err(vec![usage(
            *position,
            format!("unexpected argument `{w}`"),
            "run `via help`".to_string(),
        )]),
    };
    (command, diagnostics)
}

fn main() -> ExitCode {
    let (command, diagnostics) = parse(std::env::args_os().skip(1).collect());
    let records: Vec<Record> = match command {
        Ok(Command::Version) => {
            println!("via {VERSION}");
            Vec::new()
        }
        Ok(Command::Help) => {
            print!("{HELP}");
            Vec::new()
        }
        Err(faults) => finish(
            faults.iter().map(|d| d.emit(&NoWorld)).collect(),
            &FailOn::none(),
        ),
    };
    let mut status = exit_status(&records);
    if !records.is_empty() {
        let colour = std::env::var_os("NO_COLOR").is_none()
            && std::io::IsTerminal::is_terminal(&std::io::stderr());
        eprint!(
            "{}",
            terminal::render(
                &records,
                &[],
                Options {
                    info: false,
                    colour
                }
            )
        );
    }
    if let Some((at, file)) = diagnostics {
        if let Err(e) = std::fs::write(&file, document(&records, &[], status)) {
            // build.md 14: a file that cannot be written is `build.write_failed`, on standard
            // error only.
            let d = Diag::new(
                codes::build::WRITE_FAILED,
                Subject::Cli,
                Site::Argument(u32::try_from(at).unwrap_or(u32::MAX)),
                txt(format!(
                    "cannot write the diagnostics document `{file}`: {e}"
                )),
                text!("free space, or fix the folder's permissions"),
            )
            .emit(&NoWorld);
            eprint!("{}", terminal::render(&[d], &[], Options::default()));
            status = status.max(1);
        }
    }
    ExitCode::from(status)
}
