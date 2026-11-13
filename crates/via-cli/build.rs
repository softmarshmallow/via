//! Carries the SDK (`protocol.md` 8.1): the files of `sdk/python/viasdk/` are compiled into the
//! binary, so a build writes the SDK that matches its engine to the store before the first Python
//! step. Generated code lists each file by its path relative to `sdk/python/`, in path order.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == "__pycache__" {
            continue;
        }
        if p.is_dir() {
            walk(&p, out);
        } else if p.is_file() {
            out.push(p);
        }
    }
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let sdk = manifest.join("../../sdk/python");
    let pkg = sdk.join("viasdk");
    println!("cargo:rerun-if-changed={}", pkg.display());
    let mut files = Vec::new();
    walk(&pkg, &mut files);
    files.sort();
    let mut code = String::from("/// The carried SDK: (path under `sdk/python/`, bytes).\npub const FILES: &[(&str, &[u8])] = &[\n");
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
        let rel = f
            .strip_prefix(&sdk)
            .map(|r| {
                r.components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .unwrap_or_default();
        let _ = writeln!(
            code,
            "    ({rel:?}, include_bytes!({:?})),",
            f.canonicalize().unwrap_or(f.clone()).display().to_string()
        );
    }
    code.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default()).join("sdk_files.rs");
    let _ = std::fs::write(out, code);
}
