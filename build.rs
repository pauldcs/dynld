// build.rs — Ensure the assembly entrypoint is compiled and linked.
//
// global_asm!(include_str!("entry.s")) handles this in most cases,
// but this build script serves as a fallback and documents the intent.

use std::{env, path::PathBuf, process::Command};

const CXX_LIBTLV_DIR: &str = "cxx_libtlv";
const CXX_LIBTLV_NAME: &str = "tlv";

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut lib_path = PathBuf::from(manifest_dir);

    lib_path.push(CXX_LIBTLV_DIR);

    let status = Command::new("make")
        .current_dir(&lib_path)
        .status()
        .expect("error running `make`");

    assert!(status.success(), "make: {status}");

    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=static={}", CXX_LIBTLV_NAME);
    println!(
        "cargo:rerun-if-changed={}/lib{}.a",
        CXX_LIBTLV_DIR, CXX_LIBTLV_NAME
    );
    println!("cargo:rerun-if-changed=src/entry.s");
}
