use array::array_string::ArrayString;
use container::Container;

use crate::bindings::macho::{SELF_LIBRARY_ORDINAL, load_command, mach_header_64};

/// A Mach-O Image
///
/// This struct wraps over the image and provides methods for
/// interacting with it. It uses the Mach-O container to deserialize
/// structures from the bytes, without having to worry about endianness.
pub struct Image<'bytes> {
    /// A container container the inner Mach-O bytes.
    /// Every read/deserializing of values must be done through
    /// the methods provided by it in order to extract meaningful
    /// values.
    container: &'bytes Container<'bytes>,
}

impl<'bytes> Image<'bytes> {
    /// Creates a new Image from a given Mach-O Container
    pub fn with_container(container: &'bytes Container) -> Self {
        Self { container }
    }

    pub fn container(&self) -> &Container<'bytes> {
        self.container
    }
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub file_offset: usize,
    pub vm_addr: u64,
    pub vm_size: usize,
    pub prot: u32,
}

#[derive(Debug, Clone)]
pub enum ThreadLocalKind {
    Variable,
    Regular,
    Zerofill,
    InitFunctionPointers,
}

#[allow(unused)]
pub struct Symbol {
    lib_ordinal: u8,
    pub name: ArrayString<128>,
    pub impl_offset: usize,
    sect_num: u8,
    value: usize,
    is_weak: bool,
    is_thumb: bool,
    is_cold: bool,
}

impl Symbol {
    pub fn make_undefined(name: ArrayString<128>, lib_ordinal: u8, is_weak: bool) -> Self {
        Self {
            lib_ordinal,
            name,
            impl_offset: 0,
            sect_num: 0,
            value: 0,
            is_weak,
            is_cold: false,
            is_thumb: false,
        }
    }

    pub fn make_regular_local(
        name: ArrayString<128>,
        image_offset: usize,
        sect_num: u8,
        is_cold: bool,
        is_thumb: bool,
    ) -> Self {
        Self {
            lib_ordinal: SELF_LIBRARY_ORDINAL,
            name,
            value: 0,
            impl_offset: image_offset,
            sect_num,
            is_weak: false,
            is_cold,
            is_thumb,
        }
    }

    pub fn make_regular_export(
        name: ArrayString<128>,
        image_offset: usize,
        sect_num: u8,
        is_cold: bool,
        is_thumb: bool,
    ) -> Self {
        Self {
            lib_ordinal: SELF_LIBRARY_ORDINAL,
            name,
            value: 0,
            impl_offset: image_offset,
            sect_num,
            is_weak: false,
            is_cold,
            is_thumb,
        }
    }

    pub fn make_weak_def_export(
        name: ArrayString<128>,
        image_offset: usize,
        sect_num: u8,
        is_cold: bool,
        is_thumb: bool,
    ) -> Self {
        Self {
            lib_ordinal: SELF_LIBRARY_ORDINAL,
            name,
            value: 0,
            impl_offset: image_offset,
            sect_num,
            is_weak: true,
            is_cold,
            is_thumb,
        }
    }
}

/// A load command iterator
///
/// This iteratres over the load commands within the Image container
pub struct LCIterator<'bytes> {
    /// The container containing the Mach-O Image
    container: &'bytes Container<'bytes>,
    /// The current offset (the offset of the last returned load command)
    offset: usize,
    /// The number of load commands remaining (not yet iterated over)
    remaining: usize,
}

impl<'bytes> LCIterator<'bytes> {
    /// Creates a new load command iterator from an image container
    pub fn try_with_container(container: &'bytes Container<'bytes>) -> Result<Self, &'static str> {
        let mach_header_64 { ncmds, .. } = container
            .deserialize_type_at_offset(0)
            .map_err(|_| "error: could not create load command iterator for container")?;
        Ok(Self {
            container,
            offset: size_of::<mach_header_64>(),
            remaining: ncmds as usize,
        })
    }
}

impl<'bytes> LCIterator<'bytes> {
    pub fn try_next(&mut self) -> Result<Option<load_command>, &'static str> {
        if self.remaining == 0 {
            return Ok(None);
        }

        let load_command: load_command = self
            .container
            .deserialize_type_at_offset(self.offset)
            .map_err(|_| "error: could not iterate over load commands")?;
        self.offset += load_command.cmdsize as usize;
        self.remaining -= 1;
        Ok(Some(load_command))
    }
}

// libs/macho/src/section_type.rs (or wherever it fits)

/// Mach-O section types — the low 8 bits of the `flags` field in a section header.
/// See <mach-o/loader.h>: `SECTION_TYPE` mask is `0x000000ff`.
pub const SECTION_TYPE_MASK: u32 = 0x0000_00ff;

/// Returns a static human-readable name for a Mach-O section type.
///
/// Pass either the raw 8-bit type or the full `flags` word — the function
/// masks off the attribute bits itself.
pub fn section_type_name(flags: u32) -> &'static str {
    match flags & SECTION_TYPE_MASK {
        0x00 => "S_REGULAR",
        0x01 => "S_ZEROFILL",
        0x02 => "S_CSTRING_LITERALS",
        0x03 => "S_4BYTE_LITERALS",
        0x04 => "S_8BYTE_LITERALS",
        0x05 => "S_LITERAL_POINTERS",
        0x06 => "S_NON_LAZY_SYMBOL_POINTERS",
        0x07 => "S_LAZY_SYMBOL_POINTERS",
        0x08 => "S_SYMBOL_STUBS",
        0x09 => "S_MOD_INIT_FUNC_POINTERS",
        0x0a => "S_MOD_TERM_FUNC_POINTERS",
        0x0b => "S_COALESCED",
        0x0c => "S_GB_ZEROFILL",
        0x0d => "S_INTERPOSING",
        0x0e => "S_16BYTE_LITERALS",
        0x0f => "S_DTRACE_DOF",
        0x10 => "S_LAZY_DYLIB_SYMBOL_POINTERS",
        0x11 => "S_THREAD_LOCAL_REGULAR",
        0x12 => "S_THREAD_LOCAL_ZEROFILL",
        0x13 => "S_THREAD_LOCAL_VARIABLES",
        0x14 => "S_THREAD_LOCAL_VARIABLE_POINTERS",
        0x15 => "S_THREAD_LOCAL_INIT_FUNCTION_POINTERS",
        0x16 => "S_INIT_FUNC_OFFSETS",
        _ => "S_UNKNOWN",
    }
}
