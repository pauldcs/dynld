use crate::{
    container::Container,
    macho_image::{
        ExportTerminal, ExportsTrieSpan, MAX_REEXPORT_RESOLUTION_DEPTH, MachOImage,
        exports_trie_walk_for_symbol,
    },
};

/// Outcome of a symbol lookup in one dylib.
pub enum DylibSymbol<'a> {
    /// Defined in this dylib
    Defined { offset_from_mach_header: u64 },
    /// Re-exported from another dylib
    Reexported {
        target_install_path: &'a [u8],
        target_symbol: &'a [u8],
    },
}

/// Looks `symbol` in the container that is supposed to contain a dylib
pub fn dylib_symbol_resolve<'image>(
    container: &Container<'image>,
    symbol: &'image [u8],
) -> Result<Option<DylibSymbol<'image>>, &'static str> {
    let image = MachOImage::new(container, 0);

    let Some(ExportsTrieSpan {
        file_offset,
        size_in_bytes,
    }) = image.exports_trie_span_find()?
    else {
        return Ok(None);
    };

    let Some(trie_bytes) = container.slice(file_offset as usize, size_in_bytes as usize) else {
        return Ok(None);
    };

    let Some(terminal) = exports_trie_walk_for_symbol(trie_bytes, symbol) else {
        return Ok(None);
    };

    match terminal {
        ExportTerminal::DefinedHere {
            offset_from_mach_header,
        } => Ok(Some(DylibSymbol::Defined {
            offset_from_mach_header,
        })),
        ExportTerminal::ReexportedFromDependency {
            dependency_ordinal,
            aliased_name,
        } => {
            let Some(target_install_path) =
                image.dylib_install_path_for_ordinal(dependency_ordinal)?
            else {
                return Ok(None);
            };
            let target_symbol = if aliased_name.is_empty() {
                symbol
            } else {
                aliased_name
            };
            Ok(Some(DylibSymbol::Reexported {
                target_install_path,
                target_symbol,
            }))
        }
    }
}

/// Resolves `symbol` and chases re-exports through `library_lookup`,
/// which maps install paths to containers
pub fn dylib_symbol_resolve_following_reexports<'a, 'image: 'a, F>(
    container: &'a Container<'image>,
    symbol: &'image [u8],
    mut library_lookup: F,
) -> Result<Option<(&'image [u8], u64)>, &'static str>
where
    F: FnMut(&[u8]) -> Option<&'a Container<'image>>,
{
    let mut current_container = container;
    let mut current_symbol = symbol;
    let mut current_path: &'image [u8] = b"";

    for _ in 0..MAX_REEXPORT_RESOLUTION_DEPTH {
        match dylib_symbol_resolve(current_container, current_symbol)? {
            None => return Ok(None),
            Some(DylibSymbol::Defined {
                offset_from_mach_header,
            }) => return Ok(Some((current_path, offset_from_mach_header))),
            Some(DylibSymbol::Reexported {
                target_install_path,
                target_symbol,
            }) => {
                let Some(next) = library_lookup(target_install_path) else {
                    return Ok(None);
                };
                current_container = next;
                current_symbol = target_symbol;
                current_path = target_install_path;
            }
        }
    }

    Ok(None)
}
