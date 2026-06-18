//! Generic Mach-O image parsing: load commands, dylib install names,
//! the exports trie. Nothing in this file knows about the dyld shared
//! cache or ASLR slides.

extern crate alloc;

use alloc::vec::Vec;

use crate::bindings_dsc::{dyld_info_command, dylib_command_header};
use crate::bindings_macho::{
    EXPORT_SYMBOL_FLAGS_REEXPORT, linkedit_data_command, load_command, load_command_variants,
    mach_header_64, segment_command_64,
};
use crate::container::{Container, read_uleb128};
pub const MAX_REEXPORT_RESOLUTION_DEPTH: usize = 16;

#[derive(Debug, Clone, Copy)]
pub struct LinkeditSegment {
    pub vmaddr: u64,
    pub fileoff: u64,
}

/// One Mach-O image inside a `Container`
pub struct MachOImage<'a, 'image> {
    pub container: &'a Container<'image>,
    pub mach_header_offset: usize,
}

impl<'a, 'image> MachOImage<'a, 'image> {
    pub fn new(container: &'a Container<'image>, mach_header_offset: usize) -> Self {
        Self {
            container,
            mach_header_offset,
        }
    }

    pub fn mach_header_read(&self) -> Result<mach_header_64, &'static str> {
        self.container
            .deserialize_type_at_offset(self.mach_header_offset)
            .map_err(|_| "could not read mach_header_64")
    }

    pub fn load_commands_find<T>(
        &self,
        mut visit: impl FnMut(load_command_variants, usize) -> Result<Option<T>, &'static str>,
    ) -> Result<Option<T>, &'static str> {
        let header = self.mach_header_read()?;
        let mut command_offset = self.mach_header_offset + size_of::<mach_header_64>();

        for _ in 0..header.ncmds {
            let load_command { cmd, cmdsize, .. } = self
                .container
                .deserialize_type_at_offset(command_offset)
                .map_err(|_| "could not read load_command header")?;

            if let Some(found) = visit(cmd, command_offset)? {
                return Ok(Some(found));
            }

            command_offset += cmdsize as usize;
        }
        Ok(None)
    }

    /// Reads the install-name string out of a dylib-shaped load command.
    pub fn dylib_install_name_read(
        &self,
        command_offset: usize,
    ) -> Result<&'image [u8], &'static str> {
        let dylib_command_header { name_offset, .. } = self
            .container
            .deserialize_type_at_offset(command_offset)
            .map_err(|_| "could not read dylib_command_header")?;

        Ok(self
            .container
            .deserialize_cstr_at_offset(command_offset + name_offset as usize)
            .to_bytes())
    }

    /// File offset and size of this image's exports trie. The offset is
    /// what the load command stores verbatim, so for a DSC image the
    /// caller still has to map it through the cache's mapping tables.
    pub fn exports_trie_span_find(&self) -> Result<Option<ExportsTrieSpan>, &'static str> {
        self.load_commands_find(|cmd, command_offset| match cmd {
            load_command_variants::LC_DYLD_EXPORTS_TRIE => {
                let linkedit_data_command {
                    dataoff, datasize, ..
                } = self
                    .container
                    .deserialize_type_at_offset(command_offset)
                    .map_err(|_| "could not read linkedit_data_command")?;
                Ok(Some(ExportsTrieSpan {
                    file_offset: dataoff,
                    size_in_bytes: datasize,
                }))
            }
            load_command_variants::LC_DYLD_INFO_ONLY => {
                let dyld_info_command {
                    export_off,
                    export_size,
                    ..
                } = self
                    .container
                    .deserialize_type_at_offset(command_offset)
                    .map_err(|_| "could not read dyld_info_command")?;
                Ok(Some(ExportsTrieSpan {
                    file_offset: export_off,
                    size_in_bytes: export_size,
                }))
            }
            _ => Ok(None),
        })
    }

    pub fn dylib_install_path_for_ordinal(
        &self,
        target_ordinal: u64,
    ) -> Result<Option<&'image [u8]>, &'static str> {
        let mut seen: u64 = 0;

        self.load_commands_find(|cmd, command_offset| {
            if !is_dylib_dependency_command(cmd) {
                return Ok(None);
            }
            seen += 1;
            if seen == target_ordinal {
                Ok(Some(self.dylib_install_name_read(command_offset)?))
            } else {
                Ok(None)
            }
        })
    }

    /// Install paths of every `LC_REEXPORT_DYLIB` in declaration order
    pub fn reexported_dylib_paths_collect(&self) -> Result<Vec<&'image [u8]>, &'static str> {
        let mut paths = Vec::new();
        self.load_commands_find::<()>(|cmd, command_offset| {
            if matches!(cmd, load_command_variants::LC_REEXPORT_DYLIB) {
                if let Ok(p) = self.dylib_install_name_read(command_offset) {
                    paths.push(p);
                }
            }
            Ok(None)
        })?;
        Ok(paths)
    }

    pub fn linkedit_segment_find(&self) -> Result<Option<LinkeditSegment>, &'static str> {
        self.load_commands_find(|cmd, command_offset| match cmd {
            load_command_variants::LC_SEGMENT_64 => {
                let seg: segment_command_64 = self
                    .container
                    .deserialize_type_at_offset(command_offset)
                    .map_err(|_| "could not read segment_command_64")?;
                if &seg.segname[..10] == b"__LINKEDIT" {
                    Ok(Some(LinkeditSegment {
                        vmaddr: seg.vmaddr,
                        fileoff: seg.fileoff,
                    }))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        })
    }
}

fn is_dylib_dependency_command(cmd: load_command_variants) -> bool {
    matches!(
        cmd,
        load_command_variants::LC_LOAD_DYLIB | load_command_variants::LC_REEXPORT_DYLIB
    )
}

#[derive(Debug, Clone, Copy)]
pub struct ExportsTrieSpan {
    pub file_offset: u32,
    pub size_in_bytes: u32,
}

#[derive(Debug)]
pub enum ExportTerminal<'a> {
    /// Defined in this image at `mach_header + offset_from_mach_header`.
    DefinedHere { offset_from_mach_header: u64 },
    /// Re exported from a dependency
    ReexportedFromDependency {
        dependency_ordinal: u64,
        aliased_name: &'a [u8],
    },
}

/// Walks the exports trie at `trie_bytes` looking for `symbol`
pub fn exports_trie_walk_for_symbol<'a>(
    trie_bytes: &'a [u8],
    symbol: &[u8],
) -> Option<ExportTerminal<'a>> {
    let mut current_node_offset: usize = 0;
    let mut remaining_suffix: &[u8] = symbol;

    // A walk visits at most one node per byte of the trie; anything beyond
    // that is a cycle in malformed data. Guarantees termination.
    let mut visit_budget = trie_bytes.len();

    loop {
        if visit_budget == 0 {
            return None;
        }
        visit_budget -= 1;

        let node = TrieNodeReader::open(trie_bytes, current_node_offset)?;

        if remaining_suffix.is_empty() && node.has_terminal_payload() {
            return node.terminal_payload_read();
        }

        let (matched_label, child_node_offset) =
            node.child_edge_matching_prefix_find(remaining_suffix)?;
        remaining_suffix = &remaining_suffix[matched_label.len()..];
        current_node_offset = child_node_offset;
    }
}

struct TrieNodeReader<'a> {
    trie_bytes: &'a [u8],
    child_edges_start_offset: usize,
    terminal_payload_start_offset: usize,
    terminal_payload_size_in_bytes: u64,
}

impl<'a> TrieNodeReader<'a> {
    fn open(trie_bytes: &'a [u8], node_offset: usize) -> Option<Self> {
        if node_offset >= trie_bytes.len() {
            return None;
        }
        let (terminal_payload_size_in_bytes, terminal_size_field_byte_count) =
            read_uleb128(&trie_bytes[node_offset..])?;
        let terminal_payload_start_offset = node_offset + terminal_size_field_byte_count;
        let child_edges_start_offset =
            terminal_payload_start_offset + terminal_payload_size_in_bytes as usize;

        if child_edges_start_offset >= trie_bytes.len() {
            return None;
        }

        Some(Self {
            trie_bytes,
            child_edges_start_offset,
            terminal_payload_start_offset,
            terminal_payload_size_in_bytes,
        })
    }

    fn has_terminal_payload(&self) -> bool {
        self.terminal_payload_size_in_bytes > 0
    }

    fn terminal_payload_read(&self) -> Option<ExportTerminal<'a>> {
        let mut cursor = self.terminal_payload_start_offset;

        let (export_flags, flags_field_byte_count) = read_uleb128(&self.trie_bytes[cursor..])?;
        cursor += flags_field_byte_count;

        if export_flags & EXPORT_SYMBOL_FLAGS_REEXPORT != 0 {
            let (dependency_ordinal, ordinal_field_byte_count) =
                read_uleb128(&self.trie_bytes[cursor..])?;
            cursor += ordinal_field_byte_count;

            let aliased_name_terminator = self.trie_bytes[cursor..].iter().position(|&b| b == 0)?;
            let aliased_name = &self.trie_bytes[cursor..cursor + aliased_name_terminator];

            Some(ExportTerminal::ReexportedFromDependency {
                dependency_ordinal,
                aliased_name,
            })
        } else {
            let (offset_from_mach_header, _) = read_uleb128(&self.trie_bytes[cursor..])?;
            Some(ExportTerminal::DefinedHere {
                offset_from_mach_header,
            })
        }
    }

    fn child_edge_matching_prefix_find(
        &self,
        remaining_symbol_suffix: &[u8],
    ) -> Option<(&'a [u8], usize)> {
        let mut cursor = self.child_edges_start_offset;
        let child_edge_count = self.trie_bytes[cursor] as usize;
        cursor += 1;

        for _ in 0..child_edge_count {
            let label_start = cursor;
            let label_length = self.trie_bytes[label_start..]
                .iter()
                .position(|&b| b == 0)?;

            let label = &self.trie_bytes[label_start..label_start + label_length];

            cursor = label_start + label_length + 1;

            let (child_node_offset, offset_field_byte_count) =
                read_uleb128(&self.trie_bytes[cursor..])?;
            cursor += offset_field_byte_count;

            if !label.is_empty() && remaining_symbol_suffix.starts_with(label) {
                return Some((label, child_node_offset as usize));
            }
        }
        None
    }
}
