extern crate bindgen;
extern crate git2;
extern crate regex;

use git2::Repository;
use git2::build::CheckoutBuilder;
use std::env;
use std::fs::remove_dir_all;
use std::path::{Path, PathBuf};
use std::process::Command;
use regex::Regex;

const LIBINJECTION_URL: &'static str = "https://github.com/libinjection/libinjection";
const BUILD_DIR_NAME: &'static str = "libinjection";

fn clone_libinjection(build_dir: &Path, version: &str) -> Option<()> {
    let repo = Repository::clone(LIBINJECTION_URL, build_dir).ok()?;
    let rev = repo.revparse_single(version).ok()?;
    repo.set_head_detached(rev.id()).ok();
    let mut binding = CheckoutBuilder::new();
    let cb = binding.force();
    repo.checkout_head(Some(cb)).ok()
}

fn run_make(rule: &str, cwd: &Path) -> bool {
    let output = Command::new("make")
        .arg(rule)
        .env("OUT_DIR", env::var("OUT_DIR").unwrap())
        .current_dir(cwd)
        .output()
        .unwrap();
    if output.status.success() {
        true
    } else {
        panic!("make error: {}", String::from_utf8_lossy(&output.stderr));
    }
}

fn fix_python_version() -> Option<()> {
    if if let Ok(output) = Command::new("python").arg("-V").output() {
        let python_version = String::from_utf8_lossy(&output.stdout).to_string();
        !Regex::new("Python 2.*")
            .ok()?
            .is_match(python_version.as_str())
    } else {
        true
    } {
        let cwd = env::current_dir().ok()?;
        if !run_make("fix-python", cwd.as_path()) {
            return None;
        }
    }
    Some(())
}

fn copy_repo() -> Option<()> {

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("out_path {}", out_path.display());
    let mut build_parent_dir = out_path.join(BUILD_DIR_NAME);

    let output = Command::new("cp")
        .arg("-r")
        .arg("/Users/cp/projects/libinjection")
        .arg(build_parent_dir)
        .output()
        .unwrap();
    if output.status.success() {
        Some(())
    } else {
        panic!("make error: {}", String::from_utf8_lossy(&output.stderr));
    }
}

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut build_parent_dir = out_path.join(BUILD_DIR_NAME);

    let _ = remove_dir_all(build_parent_dir.as_path());

    println!("repo directory {}", build_parent_dir.as_path().display());

    if copy_repo().is_none() {
        panic!("unable to copy repo");
    }

    build_parent_dir.push("src");
    if !run_make("all", build_parent_dir.as_path()) {
        panic!("unable to make libinjection");
    }

    let usrlibpath = PathBuf::from("/usr/local/lib");

    println!("cargo:rustc-link-lib=static=injection");
    println!("cargo:rustc-link-search={}", usrlibpath.display());
    println!("cargo:rustc-link-search={}", build_parent_dir.display());

    let h_path = build_parent_dir.join("libinjection.h");
    let bindings = bindgen::Builder::default()
        .header(h_path.to_str().unwrap())
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("unable to write bindings");
}
