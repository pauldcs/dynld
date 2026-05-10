use core::fmt::{self, Write};

use crate::libc::{self, STDERR_FILENO, STDOUT_FILENO};

/// Attempts to write data to the object referenced by the descriptor `fildes`
/// from the buffer pointed to by `buf`
fn write_all(fd: usize, mut buf: &[u8]) -> Result<(), usize> {
    while !buf.is_empty() {
        match libc::write(fd, buf) {
            Ok(0) => return Err(0),
            Ok(n) => buf = &buf[n..],
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

struct Stdout;
struct Stderr;

impl Write for Stdout {
    #[allow(unused)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_all(STDOUT_FILENO, s.as_bytes()).map_err(|_| fmt::Error)
    }
}

impl Write for Stderr {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_all(STDERR_FILENO, s.as_bytes()).map_err(|_| fmt::Error)
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments<'_>) {
    Stdout.write_fmt(args).unwrap();
}

#[doc(hidden)]
pub fn _print_err(args: fmt::Arguments<'_>) {
    Stderr.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::print::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => { $crate::print::print!("\n") };
    ($($arg:tt)*) => {{
        $crate::print::_print(format_args!($($arg)*));
        $crate::print::_print(format_args!("\n"));
    }};
}

#[macro_export]
macro_rules! print_err {
    ($($arg:tt)*) => {
        $crate::print::_print_err(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println_err {
    () => { $crate::print::eprint!("\n") };
    ($($arg:tt)*) => {{
        $crate::print::_print_err(format_args!($($arg)*));
        $crate::print::_print_err(format_args!("\n"));
    }};
}
