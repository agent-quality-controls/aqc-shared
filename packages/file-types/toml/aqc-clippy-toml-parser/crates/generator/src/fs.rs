//! Filesystem and subprocess boundary for the generator. All `std::fs`
//! and `std::process` access lives here.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub(crate) fn load_or_fetch(cache_path: &Path, url: &str) -> String {
    if cache_path.exists() {
        println!("Using cached conf.rs at {}", cache_path.display());
        return fs::read_to_string(cache_path).expect("read cached conf.rs");
    }

    println!("Downloading {url}");
    let body = ureq::get(url)
        .call()
        .expect("download conf.rs")
        .into_string()
        .expect("read response body");

    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).expect("create cache directory");
    }
    fs::write(cache_path, &body).expect("cache conf.rs");
    body
}

/// Pipes `code` through `rustfmt` and writes the formatted output to `path`.
/// The generator commits the same code the workspace would after `cargo fmt`,
/// so subsequent `cargo fmt --check` runs stay clean across regenerations.
pub(crate) fn write_formatted_rust(path: &Path, code: &str) {
    let formatted = rustfmt(code);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create output directory");
    }
    fs::write(path, formatted).expect("write generated file");
}

fn rustfmt(code: &str) -> String {
    let mut child = Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .arg("--emit")
        .arg("stdout")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn rustfmt (is it on PATH?)");

    {
        let stdin = child.stdin.as_mut().expect("rustfmt stdin");
        stdin
            .write_all(code.as_bytes())
            .expect("write code to rustfmt stdin");
    }

    let output = child.wait_with_output().expect("collect rustfmt output");
    assert!(
        output.status.success(),
        "rustfmt failed with status {}",
        output.status
    );

    String::from_utf8(output.stdout).expect("rustfmt stdout was not utf-8")
}
