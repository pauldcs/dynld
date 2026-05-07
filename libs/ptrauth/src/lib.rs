#![no_std]

/// Sign an instruction pointer with key A.
///
/// Uses the supplied context value as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pacia(p: u64, context: u64) -> u64 {
    core::arch::naked_asm!("pacia x0, x1", "ret")
}

/// Authenticate an instruction pointer with key A.
///
/// Uses the supplied context value as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn autia(p: u64, context: u64) -> u64 {
    core::arch::naked_asm!("autia x0, x1", "ret")
}

/// Sign an instruction pointer with key B.
///
/// Uses the supplied context value as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pacib(p: u64, context: u64) -> u64 {
    core::arch::naked_asm!("pacib x0, x1", "ret")
}

/// Authenticate an instruction pointer with key B.
///
/// Uses the supplied context value as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn autib(p: u64, context: u64) -> u64 {
    core::arch::naked_asm!("autib x0, x1", "ret")
}

/// Sign a data pointer with key A.
///
/// Uses the supplied context value as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pacda(p: u64, context: u64) -> u64 {
    core::arch::naked_asm!("pacda x0, x1", "ret")
}

/// Authenticate a data pointer with key A.
///
/// Uses the supplied context value as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn autda(p: u64, context: u64) -> u64 {
    core::arch::naked_asm!("autda x0, x1", "ret")
}

/// Sign a data pointer with key B.
///
/// Uses the supplied context value as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pacdb(p: u64, context: u64) -> u64 {
    core::arch::naked_asm!("pacdb x0, x1", "ret")
}

/// Authenticate a data pointer with key B.
///
/// Uses the supplied context value as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn autdb(p: u64, context: u64) -> u64 {
    core::arch::naked_asm!("autdb x0, x1", "ret")
}

/// Sign an instruction pointer with key A.
///
/// Uses zero as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn paciza(p: u64) -> u64 {
    core::arch::naked_asm!("paciza x0", "ret")
}

/// Authenticate an instruction pointer with key A.
///
/// Uses zero as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn autiza(p: u64) -> u64 {
    core::arch::naked_asm!("autiza x0", "ret")
}

/// Sign an instruction pointer with key B.
///
/// Uses zero as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pacizb(p: u64) -> u64 {
    core::arch::naked_asm!("pacizb x0", "ret")
}

/// Authenticate an instruction pointer with key B.
///
/// Uses zero as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn autizb(p: u64) -> u64 {
    core::arch::naked_asm!("autizb x0", "ret")
}

/// Sign a data pointer with key A.
///
/// Uses zero as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pacdza(p: u64) -> u64 {
    core::arch::naked_asm!("pacdza x0", "ret")
}

/// Authenticate a data pointer with key A.
///
/// Uses zero as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn autdza(p: u64) -> u64 {
    core::arch::naked_asm!("autdza x0", "ret")
}

/// Sign a data pointer with key B.
///
/// Uses zero as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pacdzb(p: u64) -> u64 {
    core::arch::naked_asm!("pacdzb x0", "ret")
}

/// Authenticate a data pointer with key B.
///
/// Uses zero as the modifier.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn autdzb(p: u64) -> u64 {
    core::arch::naked_asm!("autdzb x0", "ret")
}

/// Generate a pointer authentication code.
///
/// Computes a PAC from the pointer and modifier without embedding it.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pacga(p: u64, context: u64) -> u64 {
    core::arch::naked_asm!("pacga x0, x0, x1", "ret")
}

// #[cfg(test)]
// mod tests {
//     use crate::*;

//     const PTR: u64 = 0x0000_1234_5678_9abc;
//     const CTX: u64 = 0xdead_beef_cafe_f00d;

//     #[test]
//     fn test_pacia_autia() {
//         unsafe {
//             assert_eq!(PTR, autia(pacia(PTR, CTX), CTX));
//         }
//     }

//     #[test]
//     fn test_pacib_autib() {
//         unsafe {
//             assert_eq!(PTR, autib(pacib(PTR, CTX), CTX));
//         }
//     }

//     #[test]
//     fn test_pacda_autda() {
//         unsafe {
//             assert_eq!(PTR, autda(pacda(PTR, CTX), CTX));
//         }
//     }

//     #[test]
//     fn test_pacdb_autdb() {
//         unsafe {
//             assert_eq!(PTR, autdb(pacdb(PTR, CTX), CTX));
//         }
//     }

//     #[test]
//     fn test_paciza_autiza() {
//         unsafe {
//             assert_eq!(PTR, autiza(paciza(PTR)));
//         }
//     }

//     #[test]
//     fn test_pacizb_autizb() {
//         unsafe {
//             assert_eq!(PTR, autizb(pacizb(PTR)));
//         }
//     }

//     #[test]
//     fn test_pacdza_autdza() {
//         unsafe {
//             assert_eq!(PTR, autdza(pacdza(PTR)));
//         }
//     }

//     #[test]
//     fn test_pacdzb_autdzb() {
//         unsafe {
//             assert_eq!(PTR, autdzb(pacdzb(PTR)));
//         }
//     }
// }
