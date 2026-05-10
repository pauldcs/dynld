//! Provides a image container
//!
//! This crate provides a way to read values from an image without having
//! to carry the burden of endianness when deserializing structures

use scroll::{BE, LE, Pread, ctx::TryFromCtx};

/// The endianness of a MachO binary
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}

impl From<Endian> for scroll::Endian {
    fn from(e: Endian) -> Self {
        match e {
            Endian::Little => LE,
            Endian::Big => BE,
        }
    }
}

/// Container for raw Mach-O binary data
///
/// This is used to help reading data from the binary.
/// Every read from the image should be done through this container
/// so data gets returned in a meaningful way.
#[derive(Debug, Clone, Copy)]
pub struct Container<'bytes> {
    /// The inner MachO bytes
    data: &'bytes [u8],

    /// The endianness of the inner binary
    pub endian: Endian,
}

impl<'bytes> Container<'bytes> {
    /// Creates a new container that wraps a given macho.
    ///
    /// It attempts to parse the magic number and determine endianness.
    /// This function panics if the first bytes don't make sense.
    pub fn with_bytes(data: &'bytes [u8], endian: Endian) -> Self {
        Self { data, endian }
    }

    /// Reads a type from the inner Macho.
    ///
    /// This function takes an offset and tries to serialize
    /// a value of the type provided (T), from the inner container.
    ///
    /// This is useful for reading values when endianness can vary
    ///
    /// the type must derive [`Pread`].
    #[inline]
    pub fn deserialize_type_at_offset<T>(&self, offset: usize) -> Result<T, ()>
    where
        T: TryFromCtx<'bytes, scroll::Endian, Error = scroll::Error>,
    {
        self.data
            .pread_with(offset, scroll::Endian::from(self.endian))
            .map_err(|_err| ())
    }

    /// Returns some inner bytes given a range
    #[inline]
    pub fn slice(&self, offset: usize, len: usize) -> Option<&'bytes [u8]> {
        self.data.get(offset..offset.checked_add(len)?)
    }

    /// returns the inner bytes
    #[inline]
    pub fn as_bytes(&self) -> &'bytes [u8] {
        self.data
    }

    /// Reads a C string from the container at the given offset.
    ///
    /// Returns a string slice up to (but not including) the null terminator.
    /// This function panics if the offset is out of bounds or if no null
    /// terminator is found.
    pub fn deserialize_string_at_offset(&self, offset: usize) -> &'bytes str {
        let bytes_from_offset = self
            .data
            .get(offset..)
            .unwrap_or_else(|| panic!("out of bounds"));

        let null_pos = bytes_from_offset
            .iter()
            .position(|&b| b == 0)
            .unwrap_or_else(|| panic!("no null terminator"));

        let cstring_bytes = &bytes_from_offset[..null_pos];

        core::str::from_utf8(cstring_bytes).unwrap_or_else(|err| panic!("invalid UTF-8: {err}"))
    }

    /// Reads a null-terminated C string from the container at the given offset,
    /// returning it as a CStr.
    ///
    /// This includes the null terminator in the returned CStr.
    /// This function panics if the offset is out of bounds or if no null
    /// terminator is found.
    pub fn deserialize_cstr_at_offset(&self, offset: usize) -> &'bytes core::ffi::CStr {
        let bytes_from_offset = self
            .data
            .get(offset..)
            .unwrap_or_else(|| panic!("out of bounds"));

        let null_pos = bytes_from_offset
            .iter()
            .position(|&b| b == 0)
            .unwrap_or_else(|| panic!("no null terminator"));

        let cstr_bytes = &bytes_from_offset[..=null_pos];

        core::ffi::CStr::from_bytes_with_nul(cstr_bytes)
            .unwrap_or_else(|err| panic!("invalid CStr: {err}"))
    }
}

/// Decode a ULEB128 from the start of `bytes`. Returns `(value, bytes_consumed)`.
pub fn read_uleb128(bytes: &[u8]) -> Option<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    for (i, &b) in bytes.iter().enumerate() {
        result |= ((b & 0x7f) as u64) << shift;
        if b & 0x80 == 0 {
            return Some((result, i + 1));
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    None
}
