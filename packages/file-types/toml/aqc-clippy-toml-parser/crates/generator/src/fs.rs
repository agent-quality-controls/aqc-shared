//! Filesystem boundary for the generator. All `std::fs` access happens here.

use std::fs;
use std::path::Path;

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

pub(crate) fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create output directory");
    }
    fs::write(path, contents).expect("write generated file");
}
