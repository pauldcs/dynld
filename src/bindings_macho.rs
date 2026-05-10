//! Provides bindings for Mach-O related constants
//!
//! Most of these are directly copied from xnu and manually
//! bound to Rust structs/enums

#[allow(dead_code)]
use scroll_derive::Pread;

type integer_t = i32;

pub type cpu_type_t = integer_t;
pub type cpu_subtype_t = integer_t;

pub const CPU_ARCH_ABI64: i32 = 0x01000000;
pub const CPU_TYPE_ARM: i32 = 12;
pub const CPU_TYPE_ARM64: i32 = CPU_TYPE_ARM | CPU_ARCH_ABI64;

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Pread)]
pub enum mach_magic {
    /// Mach-O 32-bit, little-endian
    MH_MAGIC = 0xfeedface,

    /// Mach-O 32-bit, big-endian
    MH_CIGAM = 0xcefaedfe,

    /// Mach-O 64-bit, little-endian
    MH_MAGIC_64 = 0xfeedfacf,

    /// Mach-O 64-bit, big-endian
    MH_CIGAM_64 = 0xcffaedfe,

    /// Fat binary, little-endian
    FAT_MAGIC = 0xcafebabe,

    /// Fat binary, big-endian
    FAT_CIGAM = 0xbebafeca,

    /// Fat binary with 64-bit architecture offsets, little-endian
    FAT_MAGIC_64 = 0xcafebabf,

    /// Fat binary with 64-bit architecture offsets, big-endian
    FAT_CIGAM_64 = 0xbfbafeca,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
/// The fat header appears at the beginning of fat (universal) binaries.
pub struct fat_header {
    /// FAT_MAGIC or FAT_CIGAM
    pub magic: mach_magic,

    /// number of fat_arch structs that follow
    pub nfat_arch: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
/// Describes one architecture slice in a fat binary.
pub struct fat_arch {
    /// cpu specifier
    pub cputype: cpu_type_t,

    /// machine specifier
    pub cpusubtype: cpu_subtype_t,

    /// file offset to this object file
    pub offset: u32,

    /// size of this object file
    pub size: u32,

    /// alignment as a power of 2
    pub align: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
/// The 64-bit mach header appears at the very beginning of object files for
/// 64-bit architectures.
pub struct mach_header_64 {
    /// mach magic number identifier
    pub magic: mach_magic,

    /// cpu specifier
    pub cputype: cpu_type_t,

    /// machine specifier
    pub cpusubtype: cpu_subtype_t,

    /// type of file
    pub filetype: macho_filetype_variants,

    /// number of load commands
    pub ncmds: u32,

    /// the size of all the load commands
    pub sizeofcmds: u32,

    /// flags
    pub flags: u32,

    /// reserved
    pub reserved: u32,
}

/// Constants for the filetype field of the mach_header
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Pread)]
pub enum macho_filetype_variants {
    /// relocatable object file
    MH_OBJECT = 0x1,

    /// demand paged executable file
    MH_EXECUTE = 0x2,

    /// fixed VM shared library file
    MH_FVMLIB = 0x3,

    /// core file
    MH_CORE = 0x4,

    /// preloaded executable file
    MH_PRELOAD = 0x5,

    /// dynamically bound shared library
    MH_DYLIB = 0x6,

    /// dynamic link editor
    MH_DYLINKER = 0x7,

    /// dynamically bound bundle file
    MH_BUNDLE = 0x8,

    /// shared library stub for static
    /// linking only, no section contents
    MH_DYLIB_STUB = 0x9,

    /// companion file with only debug
    /// sections
    MH_DSYM = 0xa,

    /// x86_64 kexts
    MH_KEXT_BUNDLE = 0xb,

    /// set of mach-o's
    MH_FILESET = 0xc,

    MH_GPU_EXECUTE = 0xd,

    MH_GPU_DYLIB = 0xe,
}

/// Constants for the flags field of the mach_header
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Pread)]
pub enum macho_flags_variants {
    /// the object file has no undefined
    /// references
    MH_NOUNDEFS = 0x1,

    /// the object file is the output of an
    /// incremental link against a base file
    /// and can't be link edited again
    MH_INCRLINK = 0x2,

    /// the object file is input for the
    /// dynamic linker and can't be staticly
    /// link edited again
    MH_DYLDLINK = 0x4,

    /// the object file's undefined
    /// references are bound by the dynamic
    /// linker when loaded.
    MH_BINDATLOAD = 0x8,

    /// the file has its dynamic undefined
    /// references prebound.
    MH_PREBOUND = 0x10,

    /// the file has its read-only and
    /// read-write segments split
    MH_SPLIT_SEGS = 0x20,

    /// the shared library init routine is
    /// to be run lazily via catching memory
    /// faults to its writeable segments
    /// (obsolete)
    MH_LAZY_INIT = 0x40,

    /// the image is using two-level name
    /// space bindings
    MH_TWOLEVEL = 0x80,

    /// the executable is forcing all images
    /// to use flat name space bindings
    MH_FORCE_FLAT = 0x100,

    /// this umbrella guarantees no multiple
    /// defintions of symbols in its
    /// sub-images so the two-level namespace
    /// hints can always be used.
    MH_NOMULTIDEFS = 0x200,

    /// do not have dyld notify the
    /// prebinding agent about this
    /// executable
    MH_NOFIXPREBINDING = 0x400,

    /// the binary is not prebound but can
    /// have its prebinding redone. only used
    /// when MH_PREBOUND is not set.
    MH_PREBINDABLE = 0x800,

    /// indicates that this binary binds to
    /// all two-level namespace modules of
    /// its dependent libraries. only used
    /// when MH_PREBINDABLE and MH_TWOLEVEL
    /// are both set.
    MH_ALLMODSBOUND = 0x1000,

    /// safe to divide up the sections into
    /// sub-sections via symbols for dead
    /// code stripping
    MH_SUBSECTIONS_VIA_SYMBOLS = 0x2000,

    /// the binary has been canonicalized
    /// via the unprebind operation
    MH_CANONICAL = 0x4000,

    /// the final linked image contains
    /// external weak symbols
    MH_WEAK_DEFINES = 0x8000,

    /// the final linked image uses
    /// weak symbols
    MH_BINDS_TO_WEAK = 0x10000,

    /// When this bit is set, all stacks
    /// in the task will be given stack
    /// execution privilege. Only used in
    /// MH_EXECUTE filetypes.
    MH_ALLOW_STACK_EXECUTION = 0x20000,

    /// When this bit is set, the binary
    /// declares it is safe for use in
    /// processes with uid zero
    MH_ROOT_SAFE = 0x40000,

    /// When this bit is set, the binary
    /// declares it is safe for use in
    /// processes when issetugid() is true
    MH_SETUID_SAFE = 0x80000,

    /// When this bit is set on a dylib,
    /// the static linker does not need to
    /// examine dependent dylibs to see
    /// if any are re-exported
    MH_NO_REEXPORTED_DYLIBS = 0x100000,

    /// When this bit is set, the OS will
    /// load the main executable at a
    /// random address. Only used in
    /// MH_EXECUTE filetypes.
    MH_PIE = 0x200000,

    /// Only for use on dylibs. When
    /// linking against a dylib that
    /// has this bit set, the static linker
    /// will automatically not create a
    /// LC_LOAD_DYLIB load command to the
    /// dylib if no symbols are being
    /// referenced from the dylib.
    MH_DEAD_STRIPPABLE_DYLIB = 0x400000,

    /// Contains a section of type
    /// S_THREAD_LOCAL_VARIABLES
    MH_HAS_TLV_DESCRIPTORS = 0x800000,

    /// When this bit is set, the OS will
    /// run the main executable with
    /// a non-executable heap even on
    /// platforms (e.g. i386) that don't
    /// require it. Only used in MH_EXECUTE
    /// filetypes.
    MH_NO_HEAP_EXECUTION = 0x1000000,

    /// The code was linked for use in an
    /// application extension.
    MH_APP_EXTENSION_SAFE = 0x02000000,

    /// The external symbols listed in the
    /// nlist symbol table do not include
    /// all the symbols listed in the dyld
    /// info.
    MH_NLIST_OUTOFSYNC_WITH_DYLDINFO = 0x04000000,

    /// Allow LC_MIN_VERSION_MACOS and
    /// LC_BUILD_VERSION load commands with
    /// the platforms macOS, iOSMac,
    /// iOSSimulator, tvOSSimulator and
    /// watchOSSimulator.
    MH_SIM_SUPPORT = 0x08000000,

    /// Only for use on dylibs. When this
    /// bit is set, the dylib is part of the
    /// dyld shared cache, rather than loose
    /// in the filesystem.
    MH_DYLIB_IN_CACHE = 0x80000000,
}

/// The load commands directly follow the mach_header.  The total size of all
/// of the commands is given by the sizeofcmds field in the mach_header.  All
/// load commands must have as their first two fields cmd and cmdsize.  The cmd
/// field is filled in with a constant for that command type.  Each command type
/// has a structure specifically for it.  The cmdsize field is the size in bytes
/// of the particular load command structure plus anything that follows it that
/// is a part of the load command (i.e. section structures, strings, etc.).  To
/// advance to the next load command the cmdsize can be added to the offset or
/// pointer of the current load command.  The cmdsize for 32-bit architectures
/// MUST be a multiple of 4 bytes and for 64-bit architectures MUST be a multiple
/// of 8 bytes (these are forever the maximum alignment of any load commands).
/// The padded bytes must be zero.  All tables in the object file must also
/// follow these rules so the file can be memory mapped.  Otherwise the pointers
/// to these tables will not work well or at all on some machines.  With all
/// padding zeroed like objects will compare byte for byte.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct load_command {
    /// type of load command
    pub cmd: load_command_variants,

    /// total size of command in bytes
    pub cmdsize: u32,
}

/// After MacOS X 10.1 when a new load command is added that is required to be
/// understood by the dynamic linker for the image to execute properly the
/// LC_REQ_DYLD bit will be or'ed into the load command constant. If the dynamic
/// linker sees such a load command it it does not understand will issue a
/// "unknown load command required for execution" error and refuse to use the
/// image. Other load commands without this bit that are not understood will
/// simply be ignored.
pub const LC_REQ_DYLD: u32 = 0x8000_0000;

/// Constants for the cmd field of all load commands, the type
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Pread)]
pub enum load_command_variants {
    /// segment of this file to be mapped
    LC_SEGMENT = 0x1,

    /// link-edit stab symbol table info
    LC_SYMTAB = 0x2,

    /// link-edit gdb symbol table info (obsolete)
    LC_SYMSEG = 0x3,

    /// thread
    LC_THREAD = 0x4,

    /// unix thread (includes a stack)
    LC_UNIXTHREAD = 0x5,

    /// load a specified fixed VM shared library
    LC_LOADFVMLIB = 0x6,

    /// fixed VM shared library identification
    LC_IDFVMLIB = 0x7,

    /// object identification info (obsolete)
    LC_IDENT = 0x8,

    /// fixed VM file inclusion (internal use)
    LC_FVMFILE = 0x9,

    /// prepage command (internal use)
    LC_PREPAGE = 0xa,

    /// dynamic link-edit symbol table info
    LC_DYSYMTAB = 0xb,

    /// load a dynamically linked shared library
    LC_LOAD_DYLIB = 0xc,

    /// dynamically linked shared lib ident
    LC_ID_DYLIB = 0xd,

    /// load a dynamic linker
    LC_LOAD_DYLINKER = 0xe,

    /// dynamic linker identification
    LC_ID_DYLINKER = 0xf,

    /// modules prebound for a dynamically
    /// linked shared library
    LC_PREBOUND_DYLIB = 0x10,

    /// image routines
    LC_ROUTINES = 0x11,

    /// sub framework
    LC_SUB_FRAMEWORK = 0x12,

    /// sub umbrella
    LC_SUB_UMBRELLA = 0x13,

    /// sub client
    LC_SUB_CLIENT = 0x14,

    /// sub library
    LC_SUB_LIBRARY = 0x15,

    /// two-level namespace lookup hints
    LC_TWOLEVEL_HINTS = 0x16,

    /// prebind checksum
    LC_PREBIND_CKSUM = 0x17,

    /// load a dynamically linked shared library that is allowed to be missing
    /// (all symbols are weak imported).
    LC_LOAD_WEAK_DYLIB = 0x18 | LC_REQ_DYLD,

    /// 64-bit segment of this file to be mapped
    LC_SEGMENT_64 = 0x19,

    /// 64-bit image routines
    LC_ROUTINES_64 = 0x1a,

    /// the uuid
    LC_UUID = 0x1b,

    /// runpath additions
    LC_RPATH = 0x1c | LC_REQ_DYLD,

    /// local of code signature
    LC_CODE_SIGNATURE = 0x1d,

    /// local of info to split segments
    LC_SEGMENT_SPLIT_INFO = 0x1e,

    /// load and re-export dylib
    LC_REEXPORT_DYLIB = 0x1f | LC_REQ_DYLD,

    /// delay load of dylib until first use
    LC_LAZY_LOAD_DYLIB = 0x20,

    /// encrypted segment information
    LC_ENCRYPTION_INFO = 0x21,

    /// compressed dyld information
    LC_DYLD_INFO = 0x22,

    /// compressed dyld information only
    LC_DYLD_INFO_ONLY = 0x22 | LC_REQ_DYLD,

    /// load upward dylib
    LC_LOAD_UPWARD_DYLIB = 0x23 | LC_REQ_DYLD,

    /// build for MacOSX min OS version
    LC_VERSION_MIN_MACOSX = 0x24,

    /// build for iPhoneOS min OS version
    LC_VERSION_MIN_IPHONEOS = 0x25,

    /// compressed table of function start addresses
    LC_FUNCTION_STARTS = 0x26,

    /// string for dyld to treat like environment variable
    LC_DYLD_ENVIRONMENT = 0x27,

    /// replacement for LC_UNIXTHREAD
    LC_MAIN = 0x28 | LC_REQ_DYLD,

    /// table of non-instructions in __text
    LC_DATA_IN_CODE = 0x29,

    /// source version used to build binary
    LC_SOURCE_VERSION = 0x2a,

    /// Code signing DRs copied from linked dylibs
    LC_DYLIB_CODE_SIGN_DRS = 0x2b,

    /// 64-bit encrypted segment information
    LC_ENCRYPTION_INFO_64 = 0x2c,

    /// linker options in MH_OBJECT files
    LC_LINKER_OPTION = 0x2d,

    /// optimization hints in MH_OBJECT files
    LC_LINKER_OPTIMIZATION_HINT = 0x2e,

    /// build for AppleTV min OS version
    LC_VERSION_MIN_TVOS = 0x2f,

    /// build for Watch min OS version
    LC_VERSION_MIN_WATCHOS = 0x30,

    /// arbitrary data included within a Mach-O file
    LC_NOTE = 0x31,

    /// build for platform min OS version
    LC_BUILD_VERSION = 0x32,

    /// used with linkedit_data_command, payload is trie
    LC_DYLD_EXPORTS_TRIE = 0x33 | LC_REQ_DYLD,

    /// used with linkedit_data_command
    LC_DYLD_CHAINED_FIXUPS = 0x34 | LC_REQ_DYLD,

    /// used with fileset_entry_command
    LC_FILESET_ENTRY = 0x35 | LC_REQ_DYLD,

    /// used with linkedit_data_command
    LC_ATOM_INFO = 0x36,

    /// used with linkedit_data_command
    LC_FUNCTION_VARIANTS = 0x37,

    /// used with linkedit_data_command
    LC_FUNCTION_VARIANT_FIXUPS = 0x38,

    /// target triple used to compile
    LC_TARGET_TRIPLE = 0x39,
}

impl core::ops::BitAnd<u32> for load_command_variants {
    type Output = u32;
    fn bitand(self, rhs: u32) -> Self::Output {
        (self as u32) & rhs
    }
}

/// A variable length string in a load command is represented by an lc_str
/// union.  The strings are stored just after the load command structure and
/// the offset is from the start of the load command structure.  The size
/// of the string is reflected in the cmdsize field of the load command.
/// Once again any padded bytes to bring the cmdsize field to a multiple
/// of 4 bytes must be zero.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct lc_str {
    /// offset to the string
    pub offset: u32,
}

/// The 64-bit segment load command indicates that a part of this file is to be
/// mapped into a 64-bit task's address space.  If the 64-bit segment has
/// sections then section_64 structures directly follow the 64-bit segment
/// command and their size is reflected in cmdsize.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct segment_command_64 {
    /// LC_SEGMENT_64
    pub cmd: load_command_variants,

    /// includes sizeof section_64 structs
    pub cmdsize: u32,

    /// segment name
    pub segname: [u8; 16],

    /// memory address of this segment
    pub vmaddr: u64,

    /// memory size of this segment
    pub vmsize: u64,

    /// file offset of this segment
    pub fileoff: u64,

    /// amount to map from the file
    pub filesize: u64,

    /// maximum VM protection
    pub maxprot: vm_prot_t,

    /// initial VM protection
    pub initprot: vm_prot_t,

    /// number of sections in segment
    pub nsects: u32,

    /// flags
    pub flags: u32,
}

/// Protection values, defined as bits within the vm_prot_t type
///
/// When making a new VM_PROT_*, update tests vm_parameter_validation_[user|kern]
/// and their expected results; they deliberately call VM functions with invalid
/// vm_prot values and you may be turning one of those invalid protections valid.
pub type vm_prot_t = u32;

/// No permissions
pub const VM_PROT_NONE: vm_prot_t = 0x00;

/// Read permission
pub const VM_PROT_READ: vm_prot_t = 0x01;

/// Write permission
pub const VM_PROT_WRITE: vm_prot_t = 0x02;

/// Execute permission
pub const VM_PROT_EXECUTE: vm_prot_t = 0x04;

/// The default protection for newly-created virtual memory
pub const VM_PROT_DEFAULT: vm_prot_t = VM_PROT_READ | VM_PROT_WRITE;

/// The maximum privileges possible, for parameter checking
pub const VM_PROT_ALL: vm_prot_t = VM_PROT_READ | VM_PROT_EXECUTE;

/// A segment is made up of zero or more sections.  Non-MH_OBJECT files have
/// all of their segments with the proper sections in each, and padded to the
/// specified segment alignment when produced by the link editor.  The first
/// segment of a MH_EXECUTE and MH_FVMLIB format file contains the mach_header
/// and load commands of the object file before its first section.  The zero
/// fill sections are always last in their segment (in all formats).  This
/// allows the zeroed segment padding to be mapped into memory where zero fill
/// sections might be. The gigabyte zero fill sections, those with the section
/// type S_GB_ZEROFILL, can only be in a segment with sections of this type.
/// These segments are then placed after all other segments.
///
/// The MH_OBJECT format has all of its sections in one segment for
/// compactness.  There is no padding to a specified segment boundary and the
/// mach_header and load commands are not part of the segment.
///
/// Sections with the same section name, sectname, going into the same segment,
/// segname, are combined by the link editor.  The resulting section is aligned
/// to the maximum alignment of the combined sections and is the new section's
/// alignment.  The combined sections are aligned to their original alignment in
/// the combined section.  Any padded bytes to get the specified alignment are
/// zeroed.
///
/// The format of the relocation entries referenced by the reloff and nreloc
/// fields of the section structure for mach object files is described in the
/// header file <reloc.h>.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct section_64 {
    /// name of this section
    pub sectname: [u8; 16],

    /// segment this section goes in
    pub segname: [u8; 16],

    /// memory address of this section
    pub addr: u64,

    /// size in bytes of this section
    pub size: u64,

    /// file offset of this section
    pub offset: u32,

    /// section alignment (power of 2)
    pub align: u32,

    /// file offset of relocation entries
    pub reloff: u32,

    /// number of relocation entries
    pub nreloc: u32,

    /// flags (section type and attributes)
    pub flags: u32,

    /// reserved (for offset or index)
    pub reserved1: u32,

    /// reserved (for count or sizeof)
    pub reserved2: u32,

    /// reserved
    pub reserved3: u32,
}

/*
 * The flags field of a section structure is separated into two parts a section
 * type and section attributes.  The section types are mutually exclusive (it
 * can only have one type) but the section attributes are not (it may have more
 * than one attribute).
 */
/// 256 section types
pub const SECTION_TYPE: u32 = 0x000000ff;
/// 24 section attributes
pub const SECTION_ATTRIBUTES: u32 = 0xffffff00;

/* Constants for the type of a section */

/// regular section
pub const S_REGULAR: u32 = 0x0;
/// zero fill on demand section
pub const S_ZEROFILL: u32 = 0x1;
/// section with only literal C strings
pub const S_CSTRING_LITERALS: u32 = 0x2;
/// section with only 4 byte literals
pub const S_4BYTE_LITERALS: u32 = 0x3;
/// section with only 8 byte literals
pub const S_8BYTE_LITERALS: u32 = 0x4;
/// section with only pointers to literals
pub const S_LITERAL_POINTERS: u32 = 0x5;

/*
 * For the two types of symbol pointers sections and the symbol stubs section
 * they have indirect symbol table entries.  For each of the entries in the
 * section the indirect symbol table entries, in corresponding order in the
 * indirect symbol table, start at the index stored in the reserved1 field
 * of the section structure.  Since the indirect symbol table entries
 * correspond to the entries in the section the number of indirect symbol table
 * entries is inferred from the size of the section divided by the size of the
 * entries in the section.  For symbol pointers sections the size of the entries
 * in the section is 4 bytes and for symbol stubs sections the byte size of the
 * stubs is stored in the reserved2 field of the section structure.
 */

/// section with only non-lazy symbol pointers
pub const S_NON_LAZY_SYMBOL_POINTERS: u32 = 0x6;
/// section with only lazy symbol pointers
pub const S_LAZY_SYMBOL_POINTERS: u32 = 0x7;
/// section with only symbol stubs, byte size of stub in the reserved2 field
pub const S_SYMBOL_STUBS: u32 = 0x8;
/// section with only function pointers for initialization
pub const S_MOD_INIT_FUNC_POINTERS: u32 = 0x9;
/// section with only function pointers for termination
pub const S_MOD_TERM_FUNC_POINTERS: u32 = 0xa;
/// section contains symbols that are to be coalesced
pub const S_COALESCED: u32 = 0xb;
/// zero fill on demand section (that can be larger than 4 gigabytes)
pub const S_GB_ZEROFILL: u32 = 0xc;
/// section with only pairs of function pointers for interposing
pub const S_INTERPOSING: u32 = 0xd;
/// section with only 16 byte literals
pub const S_16BYTE_LITERALS: u32 = 0xe;
/// section contains DTrace Object Format
pub const S_DTRACE_DOF: u32 = 0xf;
/// section with only lazy symbol pointers to lazy loaded dylibs
pub const S_LAZY_DYLIB_SYMBOL_POINTERS: u32 = 0x10;

#[repr(u8)]
pub enum c_void {
    __variant1,
    __variant2,
}

pub type tlv_thunk = unsafe extern "C" fn(*mut tlv_descriptor) -> *mut c_void;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tlv_descriptor {
    pub thunk: Option<tlv_thunk>,
    pub key: u64,
    pub offset: u64,
}

/*
 * Section types to support thread local variables
 */

/// template of initial values for TLVs
pub const S_THREAD_LOCAL_REGULAR: u32 = 0x11;
/// template of initial values for TLVs
pub const S_THREAD_LOCAL_ZEROFILL: u32 = 0x12;
/// TLV descriptors
pub const S_THREAD_LOCAL_VARIABLES: u32 = 0x13;
/// pointers to TLV descriptors
pub const S_THREAD_LOCAL_VARIABLE_POINTERS: u32 = 0x14;
/// functions to call to initialize TLV values
pub const S_THREAD_LOCAL_INIT_FUNCTION_POINTERS: u32 = 0x15;
/// 32-bit offsets to initializers
pub const S_INIT_FUNC_OFFSETS: u32 = 0x16;

/// Flag bits in an exports-trie terminal node.
pub const EXPORT_SYMBOL_FLAGS_KIND_MASK: u64 = 0x03;
pub const EXPORT_SYMBOL_FLAGS_REEXPORT: u64 = 0x08;
pub const EXPORT_SYMBOL_FLAGS_STUB_AND_RESOLVER: u64 = 0x10;

/*
 * Constants for the section attributes part of the flags field of a section
 * structure.
 */

/// User setable attributes
pub const SECTION_ATTRIBUTES_USR: u32 = 0xff000000;
/// section contains only true machine instructions
pub const S_ATTR_PURE_INSTRUCTIONS: u32 = 0x80000000;
/// section contains coalesced symbols that are not to be in a ranlib table of contents
pub const S_ATTR_NO_TOC: u32 = 0x40000000;
/// ok to strip static symbols in this section in files with the MH_DYLDLINK flag
pub const S_ATTR_STRIP_STATIC_SYMS: u32 = 0x20000000;
/// no dead stripping
pub const S_ATTR_NO_DEAD_STRIP: u32 = 0x10000000;
/// blocks are live if they reference live blocks
pub const S_ATTR_LIVE_SUPPORT: u32 = 0x08000000;
/// Used with i386 code stubs written on by dyld
pub const S_ATTR_SELF_MODIFYING_CODE: u32 = 0x04000000;

/*
 * If a segment contains any sections marked with S_ATTR_DEBUG then all
 * sections in that segment must have this attribute.  No section other than
 * a section marked with this attribute may reference the contents of this
 * section.  A section with this attribute may contain no symbols and must have
 * a section type S_REGULAR.  The static linker will not copy section contents
 * from sections with this attribute into its output file.  These sections
 * generally contain DWARF debugging info.
 */

/// a debug section
pub const S_ATTR_DEBUG: u32 = 0x02000000;
/// system setable attributes
pub const SECTION_ATTRIBUTES_SYS: u32 = 0x00ffff00;
/// section contains some machine instructions
pub const S_ATTR_SOME_INSTRUCTIONS: u32 = 0x00000400;
/// section has external relocation entries
pub const S_ATTR_EXT_RELOC: u32 = 0x00000200;
/// section has local relocation entries
pub const S_ATTR_LOC_RELOC: u32 = 0x00000100;

/*
 * The names of segments and sections in them are mostly meaningless to the
 * link-editor.  But there are few things to support traditional UNIX
 * executables that require the link-editor and assembler to use some names
 * agreed upon by convention.
 *
 * The initial protection of the "__TEXT" segment has write protection turned
 * off (not writeable).
 *
 * The link-editor will allocate common symbols at the end of the "__common"
 * section in the "__DATA" segment.  It will create the section and segment
 * if needed.
 */

/* The currently known segment names and the section names in those segments */

/// the pagezero segment which has no protections and catches NULL references for MH_EXECUTE files
pub const SEG_PAGEZERO: &str = "__PAGEZERO";

/// the tradition UNIX text segment
pub const SEG_TEXT: &str = "__TEXT";
/// the real text part of the text section no headers, and no padding
pub const SECT_TEXT: &str = "__text";
/// the fvmlib initialization section
pub const SECT_FVMLIB_INIT0: &str = "__fvmlib_init0";
/// the section following the fvmlib initialization section
pub const SECT_FVMLIB_INIT1: &str = "__fvmlib_init1";

/// the tradition UNIX data segment
pub const SEG_DATA: &str = "__DATA";
/// the real initialized data section no padding, no bss overlap
pub const SECT_DATA: &str = "__data";
/// the real uninitialized data section no padding
pub const SECT_BSS: &str = "__bss";
/// the section common symbols are allocated in by the link editor
pub const SECT_COMMON: &str = "__common";

/// objective-C runtime segment
pub const SEG_OBJC: &str = "__OBJC";
/// symbol table
pub const SECT_OBJC_SYMBOLS: &str = "__symbol_table";
/// module information
pub const SECT_OBJC_MODULES: &str = "__module_info";
/// string table
pub const SECT_OBJC_STRINGS: &str = "__selector_strs";
/// string table
pub const SECT_OBJC_REFS: &str = "__selector_refs";

/// the icon segment
pub const SEG_ICON: &str = "__ICON";
/// the icon headers
pub const SECT_ICON_HEADER: &str = "__header";
/// the icons in tiff format
pub const SECT_ICON_TIFF: &str = "__tiff";

/// the segment containing all structs created and maintained by the link editor. Created with -seglinkedit option to ld(1) for MH_EXECUTE and FVMLIB file types only
pub const SEG_LINKEDIT: &str = "__LINKEDIT";

/// the unix stack segment
pub const SEG_UNIXSTACK: &str = "__UNIXSTACK";

/// the segment for the self (dyld) modifing code stubs that has read, write and execute permissions
pub const SEG_IMPORT: &str = "__IMPORT";

/// Dynamicly linked shared libraries are identified by two things.  The
/// pathname (the name of the library as found for execution), and the
/// compatibility version number.  The pathname must match and the compatibility
/// number in the user of the library must be greater than or equal to the
/// library being used.  The time stamp is used to record the time a library was
/// built and copied into user so it can be use to determined if the library used
/// at runtime is exactly the same as used to built the program.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct dylib {
    /// library's path name
    pub name: lc_str,

    /// library's build time stamp
    pub timestamp: u32,

    /// library's current version number
    pub current_version: u32,

    /// library's compatibility version number
    pub compatibility_version: u32,
}

/// A dynamically linked shared library (filetype == MH_DYLIB in the mach header)
/// contains a dylib_command (cmd == LC_ID_DYLIB) to identify the library.
/// An object that uses a dynamically linked shared library also contains a
/// dylib_command (cmd == LC_LOAD_DYLIB, LC_LOAD_WEAK_DYLIB, or
/// LC_REEXPORT_DYLIB) for each library it uses.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct dylib_command {
    /// LC_ID_DYLIB, LC_LOAD_{,WEAK_}DYLIB, LC_REEXPORT_DYLIB
    pub cmd: load_command_variants,

    /// includes pathname string
    pub cmdsize: u32,

    /// the library identification
    pub dylib: dylib,
}

/// The symtab_command contains the offsets and sizes of the link-edit 4.3BSD
/// "stab" style symbol table information as described in the header files
/// <nlist.h> and <stab.h>.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct symtab_command {
    /// LC_SYMTAB
    pub cmd: load_command_variants,

    /// sizeof(struct symtab_command)
    pub cmdsize: u32,

    /// symbol table offset
    pub symoff: u32,

    /// number of symbol table entries
    pub nsyms: u32,

    /// string table offset
    pub stroff: u32,

    /// string table size in bytes
    pub strsize: u32,
}

/// This is the second set of the symbolic information which is used to support
/// the data structures for the dynamically link editor.
///
/// The original set of symbolic information in the symtab_command which contains
/// the symbol and string tables must also be present when this load command is
/// present.  When this load command is present the symbol table is organized
/// into three groups of symbols:
/// local symbols (static and debugging symbols) - grouped by module
/// defined external symbols - grouped by module (sorted by name if not lib)
/// undefined external symbols (sorted by name if MH_BINDATLOAD is not set,
/// and in order they were seen by the static linker if MH_BINDATLOAD is set)
/// In this load command there are offsets and counts to each of the three groups
/// of symbols.
///
/// This load command contains the offsets and sizes of the following new
/// symbolic information tables:
/// table of contents
/// module table
/// reference symbol table
/// indirect symbol table
///
/// For executable and object modules the information that would be in the
/// first three tables is inferred.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct dysymtab_command {
    /// LC_DYSYMTAB
    pub cmd: load_command_variants,

    /// sizeof(struct dysymtab_command)
    pub cmdsize: u32,

    /*
     * The symbols indicated by symoff and nsyms of the LC_SYMTAB load command
     * are grouped into the following three groups:
     *    local symbols (grouped by module)
     *    defined external symbols (grouped by module)
     *    undefined symbols
     */
    /// index to local symbols
    pub ilocalsym: u32,

    /// number of local symbols
    pub nlocalsym: u32,

    /// index to externally defined symbols
    pub iextdefsym: u32,

    /// number of externally defined symbols
    pub nextdefsym: u32,

    // index to undefined symbols
    pub iundefsym: u32,
    // number of undefined symbols
    pub nundefsym: u32,

    /*
     * Table of contents
     */
    pub tocoff: u32, /* file offset to table of contents */
    pub ntoc: u32,   /* number of entries in table of contents */

    /*
     * Module table
     */
    pub modtaboff: u32, /* file offset to module table */
    pub nmodtab: u32,   /* number of module table entries */

    /*
     * Referenced symbol table
     */
    pub extrefsymoff: u32, /* offset to referenced symbol table */
    pub nextrefsyms: u32,  /* number of referenced symbol table entries */

    /*
     * Indirect symbol table
     */
    pub indirectsymoff: u32, /* file offset to indirect symbol table */
    pub nindirectsyms: u32,  /* number of indirect symbol table entries */

    /*
     * External relocation entries
     */
    pub extreloff: u32, /* offset to external relocation entries */
    pub nextrel: u32,   /* number of external relocation entries */

    /*
     * Local relocation entries
     */
    pub locreloff: u32, /* offset to local relocation entries */
    pub nlocrel: u32,   /* number of local relocation entries */
}

/// The uuid load command contains a single 128-bit unique random number that
/// identifies an object produced by the static link editor.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct uuid_command {
    /// LC_UUID
    pub cmd: load_command_variants,

    /// sizeof(struct uuid_command)
    pub cmdsize: u32,

    /// the 128-bit uuid
    pub uuid: [u8; 16],
}

/// The linkedit_data_command contains the offsets and sizes of a blob
/// of data in the __LINKEDIT segment.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct linkedit_data_command {
    /// LC_CODE_SIGNATURE, LC_SEGMENT_SPLIT_INFO, LC_FUNCTION_STARTS,
    /// LC_DATA_IN_CODE, LC_DYLIB_CODE_SIGN_DRS,
    /// LC_LINKER_OPTIMIZATION_HINT, LC_DYLD_EXPORTS_TRIE,
    /// or LC_DYLD_CHAINED_FIXUPS
    pub cmd: load_command_variants,

    /// sizeof(struct linkedit_data_command)
    pub cmdsize: u32,

    /// file offset of data in __LINKEDIT segment
    pub dataoff: u32,

    /// file size of data in __LINKEDIT segment
    pub datasize: u32,
}

/// The entry_point_command is a replacement for thread_command.
/// It is used for main executables to specify the location (file offset)
/// of main().  If -stack_size was used at link time, the stacksize
/// field will contain the stack size need for the main thread.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct entry_point_command {
    /// LC_MAIN (only used in MH_EXECUTE filetypes)
    pub cmd: load_command_variants,

    /// sizeof(struct entry_point_command) (24)
    pub cmdsize: u32,

    /// file (__TEXT) offset of main()
    pub entryoff: u64,

    /// if not zero, initial stack size
    pub stacksize: u64,
}

/*
 * This is the symbol table entry structure for 64-bit architectures.
 */
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct nlist_64 {
    /*
     * In C this is a union.  For 64-bit Mach-O, it only contains n_strx.
     */
    pub n_un: nlist_64_n_un,

    /// type flag, see below
    pub n_type: u8,

    /// section number or NO_SECT
    pub n_sect: u8,

    /// see <mach-o/stab.h>
    pub n_desc: u16,

    /// value of this symbol (or stab offset)
    pub n_value: u64,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct nlist_64_n_un {
    /// index into the string table
    pub n_strx: u32,
}

/// The ordinal indicating the symbol refers to the same image in which it
/// is defined.
pub const SELF_LIBRARY_ORDINAL: u8 = 0x00;

/// The maximum valid library ordinal value.
pub const MAX_LIBRARY_ORDINAL: u8 = 0xfd;

/// The ordinal indicating the symbol should be resolved using dynamic lookup.
pub const DYNAMIC_LOOKUP_ORDINAL: u8 = 0xfe;

/// The ordinal indicating the symbol refers to the main executable.
pub const EXECUTABLE_ORDINAL: u8 = 0xff;

/// Symbols with an index into the string table of zero (n_un.n_strx == 0) are
/// defined to have a null, "", name. Therefore all string indexes to non-null
/// names must not have a zero string index. Historical note.
pub const STRIDX_NULL: u32 = 0;

/* The n_type field really contains four fields:
unsigned char N_STAB:3,
              N_PEXT:1,
              N_TYPE:3,
              N_EXT:1;
which are used via the following masks. */

/// If any of these bits set, a symbolic debugging entry
pub const N_STAB: u8 = 0xe0;

/// Private external symbol bit
pub const N_PEXT: u8 = 0x10;

/// Mask for the type bits
pub const N_TYPE: u8 = 0x0e;

/// External symbol bit, set for external symbols
pub const N_EXT: u8 = 0x01;

/*
 * Values for N_TYPE bits of the n_type field
 */

/// Undefined, n_sect == NO_SECT
pub const N_UNDF: u8 = 0x0;

/// Absolute, n_sect == NO_SECT
pub const N_ABS: u8 = 0x2;

/// Defined in section number n_sect
pub const N_SECT: u8 = 0xe;

/// Prebound undefined (defined in a dylib)
pub const N_PBUD: u8 = 0xc;

/// Indirect symbol
pub const N_INDR: u8 = 0xa;

/// If the symbol is not in any section
pub const NO_SECT: u8 = 0;

/// Maximum section number (1 through 255 inclusive)
pub const MAX_SECT: u8 = 255;

/* The bit 0x0020 of the n_desc field is used for two non-overlapping purposes
and has two different symbolic names, N_NO_DEAD_STRIP and N_DESC_DISCARDED. */

/// The N_NO_DEAD_STRIP bit of the n_desc field only ever appears in a
/// relocatable .o file (MH_OBJECT filetype). It indicates to the
/// static link editor that the symbol is never to be dead stripped.
pub const N_NO_DEAD_STRIP: u16 = 0x0020;

/// The N_DESC_DISCARDED bit of the n_desc field never appears in a linked image.
/// It is used in very rare cases by the dynamic link editor to mark an
/// in-memory symbol as discarded and no longer used for linking.
pub const N_DESC_DISCARDED: u16 = 0x0020;

/// The N_WEAK_REF bit of the n_desc field indicates to the dynamic linker that
/// the undefined symbol is allowed to be missing and is to have the address
/// zero when missing.
pub const N_WEAK_REF: u16 = 0x0040;

/// The N_WEAK_DEF bit of the n_desc field indicates to the static and dynamic
/// linkers that the symbol definition is weak, allowing a non-weak symbol to
/// be used instead, which causes the weak definition to be discarded.
/// Currently this is only supported for symbols in coalesced sections.
pub const N_WEAK_DEF: u16 = 0x0080;

/// The N_REF_TO_WEAK bit of the n_desc field indicates to the dynamic linker
/// that the undefined symbol should be resolved using flat namespace searching.
pub const N_REF_TO_WEAK: u16 = 0x0080;

/// The N_ARM_THUMB_DEF bit of the n_desc field indicates that the symbol is
/// a definition of a Thumb function.
pub const N_ARM_THUMB_DEF: u16 = 0x0008;

/// The N_SYMBOL_RESOLVER bit of the n_desc field indicates that the function
/// is actually a resolver function and should be called to get the address
/// of the real function to use. This bit is only available in .o files
/// (MH_OBJECT filetype).
pub const N_SYMBOL_RESOLVER: u16 = 0x0100;

/// The N_ALT_ENTRY bit of the n_desc field indicates that the symbol is pinned
/// to the previous content.
pub const N_ALT_ENTRY: u16 = 0x0200;

/// The N_COLD_FUNC bit of the n_desc field indicates that the symbol is used
/// infrequently and the linker should order it towards the end of the section.
pub const N_COLD_FUNC: u16 = 0x0400;

/*
 * For images created by the static link editor with the -twolevel_namespace
 * option in effect the flags field of the mach header is marked with
 * MH_TWOLEVEL.  And the binding of the undefined references of the image are
 * determined by the static link editor.  Which library an undefined symbol is
 * bound to is recorded by the static linker in the high 8 bits of the n_desc
 * field using the SET_LIBRARY_ORDINAL macro below.  The ordinal recorded
 * references the libraries listed in the Mach-O's LC_LOAD_DYLIB,
 * LC_LOAD_WEAK_DYLIB, LC_REEXPORT_DYLIB, LC_LOAD_UPWARD_DYLIB, and
 * LC_LAZY_LOAD_DYLIB, etc. load commands in the order they appear in the
 * headers.   The library ordinals start from 1.
 *
 * For a dynamic library that is built as a two-level namespace image the
 * undefined references from module defined in another use the same nlist struct
 * an in that case SELF_LIBRARY_ORDINAL is used as the library ordinal.  For
 * defined symbols in all images they also must have the library ordinal set to
 * SELF_LIBRARY_ORDINAL.  The EXECUTABLE_ORDINAL refers to the executable
 * image for references from plugins that refer to the executable that loads
 * them.
 *
 * The DYNAMIC_LOOKUP_ORDINAL is for undefined symbols in a two-level namespace
 * image that are looked up by the dynamic linker with flat namespace semantics.
 * This ordinal was added as a feature in Mac OS X 10.3 by reducing the
 * value of MAX_LIBRARY_ORDINAL by one.  So it is legal for existing binaries
 * or binaries built with older tools to have 0xfe (254) dynamic libraries.  In
 * this case the ordinal value 0xfe (254) must be treated as a library ordinal
 * for compatibility.
 */

/*
 * GET_LIBRARY_ORDINAL(n_desc)
 *   (((n_desc) >> 8) & 0xff)
 */
#[inline]
pub const fn get_library_ordinal(n_desc: u16) -> u8 {
    ((n_desc >> 8) & 0xff) as u8
}

/*
 * SET_LIBRARY_ORDINAL(n_desc, ordinal)
 *   (n_desc) = (((n_desc) & 0x00ff) | (((ordinal) & 0xff) << 8))
 */
#[inline]
pub fn set_library_ordinal(n_desc: &mut u16, ordinal: u8) {
    *n_desc = (*n_desc & 0x00ff) | ((ordinal as u16) << 8);
}

/// Header of the LC_DYLD_CHAINED_FIXUPS payload
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct dyld_chained_fixups_header {
    /// fixups version (currently 0)
    pub fixups_version: u32,

    /// offset of dyld_chained_starts_in_image in chain_data
    pub starts_offset: u32,

    /// offset of imports table in chain_data
    pub imports_offset: u32,

    /// offset of symbol strings in chain_data
    pub symbols_offset: u32,

    /// number of imported symbol names
    pub imports_count: u32,

    /// DYLD_CHAINED_IMPORT*
    pub imports_format: dyld_chained_import_format_variants,

    /// 0 => uncompressed, 1 => zlib compressed
    pub symbols_format: u32,
}

/// This struct is embedded in LC_DYLD_CHAINED_FIXUPS payload
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct dyld_chained_starts_in_image {
    pub seg_count: u32,
    // each entry is offset into this struct for that segment
    // pub seg_info_offset: [u32; 16],
    // followed by pool of dyld_chained_starts_in_segment data
}

/// This struct is embedded in dyld_chain_starts_in_image
/// and passed down to the kernel for page-in linking
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct dyld_chained_starts_in_segment {
    /// size of this (amount kernel needs to copy)
    pub size: u32,

    /// 0x1000 or 0x4000
    pub page_size: u16,

    /// DYLD_CHAINED_PTR_*
    pub pointer_format: dyld_chained_ptr_format_variants,

    /// offset in memory to start of segment
    pub segment_offset: u64,

    /// for 32-bit OS, any value beyond this is not a pointer
    pub max_valid_pointer: u32,

    /// how many pages are in array
    pub page_count: u16,

    /// each entry is offset in each page of first element in chain
    /// or DYLD_CHAINED_PTR_START_NONE if no fixups on page
    pub page_start: [u16; 1],
    // followed by optional chain_starts[]
}

#[repr(u16)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Pread)]
pub enum dyld_chained_ptr_start_variants {
    /// used in page_start[] to denote a page with no fixups
    DYLD_CHAINED_PTR_START_NONE = 0xFFFF,

    /// used in page_start[] to denote a page which has multiple starts
    DYLD_CHAINED_PTR_START_MULTI = 0x8000,
    // used in chain_starts[] to denote last start in list for page
    // DYLD_CHAINED_PTR_START_LAST = 0x8000
}

/// This struct is embedded in __TEXT,__chain_starts section in firmware
#[repr(C)]
#[derive(Debug, Copy, Clone, Pread)]
pub struct dyld_chained_starts_offsets {
    /// DYLD_CHAINED_PTR_32_FIRMWARE or DYLD_CHAINED_PTR_ARM64E_KERNEL
    pub pointer_format: u32,

    /// number of starts in array
    pub starts_count: u32,

    /// array of chain start offsets
    pub chain_starts: [u32; 1],
}

/// Values for dyld_chained_starts_in_segment.pointer_format
#[repr(u16)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Pread)]
pub enum dyld_chained_ptr_format_variants {
    /// stride 8, unauth target is vmaddr
    DYLD_CHAINED_PTR_ARM64E = 1,

    /// target is vmaddr
    DYLD_CHAINED_PTR_64 = 2,

    DYLD_CHAINED_PTR_32 = 3,
    DYLD_CHAINED_PTR_32_CACHE = 4,
    DYLD_CHAINED_PTR_32_FIRMWARE = 5,

    /// target is vm offset
    DYLD_CHAINED_PTR_64_OFFSET = 6,

    /// old name
    DYLD_CHAINED_PTR_ARM64E_OFFSET = 7,

    /// stride 4, unauth target is vm offset
    /// DYLD_CHAINED_PTR_ARM64E_KERNEL = 7,
    DYLD_CHAINED_PTR_64_KERNEL_CACHE = 8,

    /// stride 8, unauth target is vm offset
    DYLD_CHAINED_PTR_ARM64E_USERLAND = 9,

    /// stride 4, unauth target is vmaddr
    DYLD_CHAINED_PTR_ARM64E_FIRMWARE = 10,

    /// stride 1, x86_64 kernel caches
    DYLD_CHAINED_PTR_X86_64_KERNEL_CACHE = 11,

    /// stride 8, unauth target is vm offset, 24-bit bind
    DYLD_CHAINED_PTR_ARM64E_USERLAND24 = 12,

    /// stride 8, regular/auth targets both vm offsets. Only A keys supported
    DYLD_CHAINED_PTR_ARM64E_SHARED_CACHE = 13,

    /// stride 4, rebase offsets use segIndex and segOffset
    DYLD_CHAINED_PTR_ARM64E_SEGMENTED = 14,
}

pub const BIND_SPECIAL_DYLIB_SELF: u32 = 0;
pub const BIND_SPECIAL_DYLIB_MAIN_EXECUTABLE: u32 = 255;
pub const BIND_SPECIAL_DYLIB_FLAT_LOOKUP: u32 = 254;
pub const BIND_SPECIAL_DYLIB_WEAK_LOOKUP: u32 = 253;

/// Values for dyld_chained_fixups_header.imports_format
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Pread)]
pub enum dyld_chained_import_format_variants {
    DYLD_CHAINED_IMPORT = 1,
    DYLD_CHAINED_IMPORT_ADDEND = 2,
    DYLD_CHAINED_IMPORT_ADDEND64 = 3,
}

// DYLD_CHAINED_IMPORT_ADDEND64
#[repr(C)]
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_import_addend64 {
    /// packed import fields
    pub header: dyld_chained_import,

    /// addend applied to the symbol
    pub addend: u64,
}

/// DYLD_CHAINED_PTR_ARM64E
/// bits:
/// [0..=42]  target
/// [43..=50] high8
/// [51..=61] next        (4- or 8-byte stride)
/// [62]      bind = 0
/// [63]      auth = 0
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_ptr_arm64e_rebase(pub u64);
impl dyld_chained_ptr_arm64e_rebase {
    #[inline]
    pub fn target(&self) -> u64 {
        self.0 & 0x0000_07FF_FFFF_FFFF
    }
    #[inline]
    pub fn high8(&self) -> u8 {
        ((self.0 >> 43) & 0xFF) as u8
    }
    #[inline]
    pub fn next(&self) -> u16 {
        ((self.0 >> 51) & 0x7FF) as u16
    }
}

/// DYLD_CHAINED_PTR_ARM64E
/// bits:
/// [0..=15]  ordinal
/// [16..=31] zero
/// [32..=50] addend     (+/-256K)
/// [51..=61] next       (4- or 8-byte stride)
/// [62]      bind = 1
/// [63]      auth = 0
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_ptr_arm64e_bind(pub u64);
impl dyld_chained_ptr_arm64e_bind {
    #[inline]
    pub fn ordinal(&self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    #[inline]
    pub fn addend(&self) -> i32 {
        ((self.0 >> 32) & 0x7FFFF) as i32
    }

    #[inline]
    pub fn next(&self) -> u16 {
        ((self.0 >> 51) & 0x7FF) as u16
    }
}

/// DYLD_CHAINED_PTR_ARM64E
/// bits:
/// [0..=31]  target     (runtimeOffset)
/// [32..=47] diversity
/// [48]      addrDiv
/// [49..=50] key
/// [51..=61] next       (4- or 8-byte stride)
/// [62]      bind = 0
/// [63]      auth = 1
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_ptr_arm64e_auth_rebase(pub u64);
impl dyld_chained_ptr_arm64e_auth_rebase {
    #[inline]
    pub fn target(&self) -> u32 {
        (self.0 & 0xFFFF_FFFF) as u32
    }

    #[inline]
    pub fn diversity(&self) -> u16 {
        ((self.0 >> 32) & 0xFFFF) as u16
    }

    #[inline]
    pub fn addr_div(&self) -> bool {
        (self.0 & (1 << 48)) != 0
    }

    #[inline]
    pub fn key(&self) -> u8 {
        ((self.0 >> 49) & 0x3) as u8
    }

    #[inline]
    pub fn next(&self) -> u16 {
        ((self.0 >> 51) & 0x7FF) as u16
    }
}

/// DYLD_CHAINED_PTR_ARM64E
/// bits:
/// [0..=15]  ordinal
/// [16..=31] zero
/// [32..=47] diversity
/// [48]      addrDiv
/// [49..=50] key
/// [51..=61] next       (4- or 8-byte stride)
/// [62]      bind = 1
/// [63]      auth = 1
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_ptr_arm64e_auth_bind(pub u64);
impl dyld_chained_ptr_arm64e_auth_bind {
    #[inline]
    pub fn ordinal(&self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    #[inline]
    pub fn diversity(&self) -> u16 {
        ((self.0 >> 32) & 0xFFFF) as u16
    }

    #[inline]
    pub fn addr_div(&self) -> bool {
        (self.0 & (1 << 48)) != 0
    }

    #[inline]
    pub fn key(&self) -> u8 {
        ((self.0 >> 49) & 0x3) as u8
    }

    #[inline]
    pub fn next(&self) -> u16 {
        ((self.0 >> 51) & 0x7FF) as u16
    }
}

/// DYLD_CHAINED_PTR_64 / DYLD_CHAINED_PTR_64_OFFSET
/// bits:
/// [0..=35]  target
/// [36..=43] high8
/// [44..=50] reserved = 0
/// [51..=62] next       (4-byte stride)
/// [63]      bind = 0
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_ptr_64_rebase(pub u64);
impl dyld_chained_ptr_64_rebase {
    #[inline]
    pub fn target(&self) -> u64 {
        self.0 & 0x0000_000F_FFFF_FFFF
    }

    #[inline]
    pub fn high8(&self) -> u8 {
        ((self.0 >> 36) & 0xFF) as u8
    }

    #[inline]
    pub fn next(&self) -> u16 {
        ((self.0 >> 51) & 0xFFF) as u16
    }
}

/// DYLD_CHAINED_PTR_64
/// bits:
/// [0..=23]  ordinal
/// [24..=31] addend     (0..255)
/// [32..=50] reserved = 0
/// [51..=62] next       (4-byte stride)
/// [63]      bind = 1
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_ptr_64_bind(pub u64);
impl dyld_chained_ptr_64_bind {
    #[inline]
    pub fn ordinal(&self) -> u32 {
        (self.0 & 0x00FF_FFFF) as u32
    }

    #[inline]
    pub fn addend(&self) -> u8 {
        ((self.0 >> 24) & 0xFF) as u8
    }

    #[inline]
    pub fn next(&self) -> u16 {
        ((self.0 >> 51) & 0xFFF) as u16
    }
}

/// DYLD_CHAINED_PTR_ARM64E_USERLAND24
/// bits:
/// [0..=23]  ordinal
/// [24..=31] zero
/// [32..=50] addend     (+/-256K)
/// [51..=61] next       (8-byte stride)
/// [62]      bind = 1
/// [63]      auth = 0
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_ptr_arm64e_bind_24(pub u64);
impl dyld_chained_ptr_arm64e_bind_24 {
    #[inline]
    pub fn ordinal(&self) -> u32 {
        (self.0 & 0x00FF_FFFF) as u32
    }

    #[inline]
    pub fn addend(&self) -> i32 {
        ((self.0 >> 32) & 0x7FFFF) as i32
    }

    #[inline]
    pub fn next(&self) -> u16 {
        ((self.0 >> 51) & 0x7FF) as u16
    }
}

/// DYLD_CHAINED_PTR_ARM64E_USERLAND24
/// bits:
/// [0..=23]  ordinal
/// [24..=31] zero
/// [32..=47] diversity
/// [48]      addrDiv
/// [49..=50] key
/// [51..=61] next       (8-byte stride)
/// [62]      bind = 1
/// [63]      auth = 1
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_ptr_arm64e_auth_bind_24(pub u64);
impl dyld_chained_ptr_arm64e_auth_bind_24 {
    #[inline]
    pub fn ordinal(&self) -> u32 {
        (self.0 & 0x00FF_FFFF) as u32
    }

    #[inline]
    pub fn diversity(&self) -> u16 {
        ((self.0 >> 32) & 0xFFFF) as u16
    }

    #[inline]
    pub fn addr_div(&self) -> bool {
        (self.0 & (1 << 48)) != 0
    }

    #[inline]
    pub fn key(&self) -> u8 {
        ((self.0 >> 49) & 0x3) as u8
    }

    #[inline]
    pub fn next(&self) -> u16 {
        ((self.0 >> 51) & 0x7FF) as u16
    }
}

/// DYLD_CHAINED_PTR_ARM64E_SEGMENTED
/// low 32 bits:
///   [0..=27] targetSegOffset
///   [28..=31] targetSegIndex
/// high 32 bits:
///   [0..=18] padding
///   [19..=30] next      (4-byte stride)
///   [31]      auth = 0
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_ptr_arm64e_segmented_rebase(pub u64);
impl dyld_chained_ptr_arm64e_segmented_rebase {
    #[inline]
    pub fn target_seg_offset(&self) -> u32 {
        (self.0 & 0x0FFF_FFFF) as u32
    }

    #[inline]
    pub fn target_seg_index(&self) -> u8 {
        ((self.0 >> 28) & 0xF) as u8
    }

    #[inline]
    pub fn next(&self) -> u16 {
        ((self.0 >> 51) & 0xFFF) as u16
    }
}

/// DYLD_CHAINED_PTR_ARM64E_SEGMENTED
/// low 32 bits:
///   [0..=27] targetSegOffset
///   [28..=31] targetSegIndex
/// high 32 bits:
///   [0..=15] diversity
///   [16]      addrDiv
///   [17..=18] key
///   [19..=30] next      (4-byte stride)
///   [31]      auth = 1
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_ptr_arm64e_auth_segmented_rebase(pub u64);
impl dyld_chained_ptr_arm64e_auth_segmented_rebase {
    #[inline]
    pub fn target_seg_offset(&self) -> u32 {
        (self.0 & 0x0FFF_FFFF) as u32
    }

    #[inline]
    pub fn target_seg_index(&self) -> u8 {
        ((self.0 >> 28) & 0xF) as u8
    }

    #[inline]
    pub fn diversity(&self) -> u16 {
        ((self.0 >> 32) & 0xFFFF) as u16
    }

    #[inline]
    pub fn addr_div(&self) -> bool {
        (self.0 & (1 << 48)) != 0
    }

    #[inline]
    pub fn key(&self) -> u8 {
        ((self.0 >> 49) & 0x3) as u8
    }

    #[inline]
    pub fn next(&self) -> u16 {
        ((self.0 >> 51) & 0xFFF) as u16
    }
}

/// DYLD_CHAINED_PTR_64_KERNEL_CACHE
/// bits:
/// [0..=29]  target
/// [30..=31] cacheLevel
/// [32..=47] diversity
/// [48]      addrDiv
/// [49..=50] key
/// [51..=62] next       (1- or 4-byte stride)
/// [63]      isAuth
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_ptr_64_kernel_cache_rebase(pub u64);
impl dyld_chained_ptr_64_kernel_cache_rebase {
    #[inline]
    pub fn target(&self) -> u32 {
        (self.0 & 0x3FFF_FFFF) as u32
    }

    #[inline]
    pub fn cache_level(&self) -> u8 {
        ((self.0 >> 30) & 0x3) as u8
    }

    #[inline]
    pub fn diversity(&self) -> u16 {
        ((self.0 >> 32) & 0xFFFF) as u16
    }

    #[inline]
    pub fn addr_div(&self) -> bool {
        (self.0 & (1 << 48)) != 0
    }

    #[inline]
    pub fn key(&self) -> u8 {
        ((self.0 >> 49) & 0x3) as u8
    }

    #[inline]
    pub fn next(&self) -> u16 {
        ((self.0 >> 51) & 0xFFF) as u16
    }

    #[inline]
    pub fn is_auth(&self) -> bool {
        (self.0 & (1 << 63)) != 0
    }
}

/// DYLD_CHAINED_PTR_ARM64E_SHARED_CACHE
/// bits:
/// [0..=33]  runtimeOffset
/// [34..=41] high8
/// [42..=51] unused
/// [52..=62] next       (8-byte stride)
/// [63]      auth = 0
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_ptr_arm64e_shared_cache_rebase(pub u64);
impl dyld_chained_ptr_arm64e_shared_cache_rebase {
    #[inline]
    pub fn runtime_offset(&self) -> u64 {
        self.0 & 0x0000_0003_FFFF_FFFF
    }

    #[inline]
    pub fn high8(&self) -> u8 {
        ((self.0 >> 34) & 0xFF) as u8
    }

    #[inline]
    pub fn next(&self) -> u16 {
        ((self.0 >> 52) & 0x7FF) as u16
    }
}

/// DYLD_CHAINED_IMPORT
/// bits:
/// [0..=7]   libOrdinal
/// [8]       weakImport
/// [9..=31]  nameOffset
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_import(pub u32);
impl dyld_chained_import {
    #[inline]
    pub fn lib_ordinal(&self) -> u8 {
        (self.0 & 0x0000_00FF) as u8
    }

    #[inline]
    pub fn weak_import(&self) -> bool {
        (self.0 & 0x0000_0100) != 0
    }

    #[inline]
    pub fn name_offset(&self) -> u32 {
        (self.0 & 0xFFFF_FE00) >> 9
    }
}

/// DYLD_CHAINED_IMPORT_ADDEND
/// same layout as DyldChainedImport
/// followed by i32 addend
#[derive(Copy, Clone, Debug, Pread)]
pub struct dyld_chained_import_addend {
    pub header: dyld_chained_import,
    pub addend: i32,
}
