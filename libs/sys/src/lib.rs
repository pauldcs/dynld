//! Darwin/XNU syscall primitives for aarch64/x86.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(clippy::missing_safety_doc)]

pub mod file;
pub mod syscalls;
pub mod unistd;

#[allow(non_camel_case_types)]
pub mod mach;

#[allow(non_camel_case_types)]
pub mod mmap;

pub use unistd::{close, exit, fstat, open, read, write};

const SYS_EXIT: usize = 1;
const SYS_WRITE: usize = 4;
const SYS_READ: usize = 3;
const SYS_OPEN: usize = 5;
const SYS_CLOSE: usize = 6;
const SYS_FSTAT: usize = 339;
const SYS_MMAP: usize = 197;
const SYS_MUNMAP: usize = 73;
const SYS_MPROTECT: usize = 74;
