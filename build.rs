// build.rs — Ensure the assembly entrypoint is compiled and linked.
//
// global_asm!(include_str!("entry.s")) handles this in most cases,
// but this build script serves as a fallback and documents the intent.

use std::{env, path::PathBuf, process::Command};

const LIBTLV_DIR: &str = "libtlv";
const LIBTLV_NAME: &str = "tlv";
const LIBDSC_DIR: &str = "libdsc";
const LIBDSC_NAME: &str = "dsc";

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let mut lib_tlv_path = PathBuf::from(&manifest_dir);
    lib_tlv_path.push(LIBTLV_DIR);
    let status = Command::new("make")
        .current_dir(&lib_tlv_path)
        .status()
        .expect("error running `make` on libtlv");
    assert!(status.success(), "make: {status}");

    println!("cargo:rustc-link-lib=static={}", LIBTLV_NAME);
    println!("cargo:rerun-if-changed={}/lib{}.a", LIBTLV_DIR, LIBTLV_NAME);
    println!("cargo:rustc-link-search=native={}", lib_tlv_path.display());

    let mut lib_dsc_path = PathBuf::from(&manifest_dir);
    lib_dsc_path.push(LIBDSC_DIR);
    let status = Command::new("make")
        .current_dir(&lib_dsc_path)
        .status()
        .expect("error running `make` on libtlv");

    assert!(status.success(), "make: {status}");
    println!("cargo:rustc-link-lib=static={}", LIBDSC_NAME);
    println!("cargo:rerun-if-changed={}/lib{}.a", LIBDSC_DIR, LIBDSC_NAME);
    println!("cargo:rustc-link-search=native={}", lib_dsc_path.display());

    println!("cargo:rerun-if-changed=src/entry.s");
}
