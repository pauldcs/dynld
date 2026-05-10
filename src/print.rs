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

// pub const HEX: &str = "0123456789abcdef";
// pub fn print_ptr(ptr: *const u8) {
//     let mut buf = [0u8; 19]; // "0x" + 16 hex digits + "\n"
//     buf[0] = b'0';
//     buf[1] = b'x';

//     let addr = ptr as usize;
//     let hex = HEX.as_bytes();

//     // Fill 16 hex digits, most-significant nibble first.
//     let mut i = 0;
//     while i < 16 {
//         let shift = (15 - i) * 4;
//         let nibble = (addr >> shift) & 0xf;
//         buf[2 + i] = hex[nibble];
//         i += 1;
//     }

//     let _ = write(2, &buf);
// }
