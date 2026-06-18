use core::ptr::NonNull;

use alloc::{string::ToString, vec::Vec};

use crate::{
    DYLD_LIKELY_UNSLID_VM_ADDR, LIBSYSTEM_PATH,
    bindings_macho::{CPU_TYPE_ARM64, fat_arch, fat_header, mach_header_64, mach_magic},
    container::Container,
    dsc,
    dyld_shared_cache::DyldSharedCache,
    fixups::{Fixup, FixupKind, fixup_all_chained_fixups},
    image::Symbol,
    jump,
    libc::exit_error,
    loader::{Library, macho_endian_from_magic, macho_loader},
    println_err,
};

/// Extracts the directory portion of a path, dropping the final component.
pub fn dir_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(0) => "/",
        Some(i) => &path[..i],
        None => ".",
    }
}

/// Reads `argv[0]` as a null-terminated UTF-8 string
///
/// # Safety
/// `argv` must point to a non-null, null terminated array of C strings,
/// as provided by main in C
pub unsafe fn argv0<'a>(argv: *const *const u8) -> Option<&'a str> {
    unsafe {
        let arg0 = *argv;
        if arg0.is_null() {
            return None;
        }

        let mut len = 0;
        while *arg0.add(len) != 0 {
            len += 1;
        }

        str::from_utf8(core::slice::from_raw_parts(arg0, len)).ok()
    }
}

/// Extracts the directory containing the executable from a C style argv
///
/// # Safety
/// See [`argv0`].
pub unsafe fn exe_dir<'a>(argv: *const *const u8) -> Option<&'a str> {
    unsafe { argv0(argv).map(dir_of) }
}

/// The main entrypoint of dynld
pub fn dynld_entrypoint(
    self_bind_fixups: Vec<Fixup>,
    dylinker_container: Container<'_>,
    executable_header: *mut mach_header_64,
    executable_size: usize,
    argc: usize,
    argv: *const *const u8,
    envp: *const *const u8,
    apple: *const *const u8,
) -> ! {
    let image_raw_bytes =
        unsafe { core::slice::from_raw_parts(executable_header as *const u8, executable_size) };

    let image_mach_magic =
        unsafe { core::ptr::read_unaligned(executable_header as *const mach_magic) };

    let image = arm64_container_create(&Container::with_bytes(
        image_raw_bytes,
        macho_endian_from_magic(image_mach_magic),
    ))
    .map_err(|err: &str| {
        println_err!("failed to create an arm64 container: {err}");
        exit_error()
    })
    .unwrap();

    let mut dyld_shared_cache_baddr_in_vm = 0;
    if unsafe { dsc::dyld_shared_cache_base_address_get(&mut dyld_shared_cache_baddr_in_vm) } == 0 {
        println_err!("failed to find the dyld_shared_cache base address");
        exit_error()
    }

    let mut dyld_shared_cache = DyldSharedCache::new();
    if let Err(err) = unsafe {
        dyld_shared_cache.from_existing(
            dyld_shared_cache_baddr_in_vm as *const u8,
            dyld_shared_cache_baddr_in_vm - DYLD_LIKELY_UNSLID_VM_ADDR,
        )
    } {
        println_err!("failed initializing dyld shared cache: '{err}'");
        exit_error()
    }

    if let Err(err) =
        tls_functions_self_bind_hack(&dylinker_container, self_bind_fixups, &dyld_shared_cache)
    {
        println_err!("failed to bind TLS function to self: '{err}'");
        exit_error()
    }

    let exec_dir = unsafe {
        exe_dir(argv).unwrap_or_else(|| {
            println_err!("failed to get the exeutable path of the program");
            exit_error()
        })
    };

    let (_vm, entrypoint) =
        macho_loader(&dyld_shared_cache, image, exec_dir, None).unwrap_or_else(|err| {
            println_err!("dylinking error: '{err}', giving up...");
            exit_error()
        });

    //println!("[+] dylinking OK, jumping to 0x{entrypoint:x}");
    // jump to the entrypoint. We should make sure that this is OK to do
    // before doing this, we fully expect everything to work
    unsafe {
        jump::entry_and_ret(
            NonNull::new_unchecked(entrypoint as *mut u8),
            argc,
            argv,
            envp,
            apple,
        )
    };

    // THIS FUNCTION CANNOT RETURN! the caller nulled the link register
    // and the frame pointer, we try to exit but trigger and inifite loop
    // if that does not work

    exit_error()
}

/// This function takes the fixups required for TLS to work.
///
/// The functions in `libtlv` handle thread local variables, but they
/// require libraries that we do not yet link to. We extracted the fixups
/// previously and now we apply them.
///
/// This isn't a very pleasant way to do so but for now this does the job
fn tls_functions_self_bind_hack(
    self_container: &Container<'_>,
    mut self_bind_fixups: Vec<Fixup>,
    dyld_shared_cache: &DyldSharedCache<'_>,
) -> Result<(), &'static str> {
    let mut self_bind_symbols = Vec::new();
    let mut self_bind_dylibs = Vec::new();
    self_bind_dylibs.push(Library::System(LIBSYSTEM_PATH.to_string()));

    for bind in self_bind_fixups.iter_mut() {
        match &mut bind.kind {
            FixupKind::Bind {
                symbol_name,
                ordinal,
                ..
            } => {
                *ordinal = 1;
                self_bind_symbols.push(Symbol::make_undefined(symbol_name.clone(), 1, false));
            }
            _ => (),
        }
    }

    if let Err(err) = unsafe {
        fixup_all_chained_fixups(
            self_container.as_bytes().as_ptr() as *mut u8,
            0,
            &self_bind_fixups,
            &self_bind_symbols,
            &self_bind_dylibs,
            Some(&dyld_shared_cache),
            false,
        )
    } {
        return Err(err);
    };

    Ok(())
}

/// Tries to find an image with a `CPU_TYPE_ARM64` cputype
///
/// If the container is a fat_magic image, we try to find
/// the correct architecture within it
fn arm64_container_create<'bytes>(
    image_container: &Container<'bytes>,
) -> Result<Container<'bytes>, &'static str> {
    let header = image_container
        .deserialize_type_at_offset::<mach_header_64>(0)
        .unwrap();

    // if the image already has a CPU_TYPE_ARM64 cputype,
    // we can return it
    if header.magic == mach_magic::MH_MAGIC_64 && header.cputype == CPU_TYPE_ARM64 {
        return Ok(*image_container);
    }

    // if this is not a fat binary and is not a CPU_TYPE_ARM64,
    // we return an error
    if header.magic != mach_magic::FAT_MAGIC {
        return Err("Not a valid ARM64 Mach-O or fat binary");
    }

    let fat_header = image_container
        .deserialize_type_at_offset::<fat_header>(0)
        .unwrap();
    (0..fat_header.nfat_arch)
        .filter_map(|index| {
            let offset = size_of::<fat_header>() + index as usize * size_of::<fat_arch>();

            let fat_arch { offset, size, .. } = image_container
                .deserialize_type_at_offset::<fat_arch>(offset)
                .ok()?;

            image_container
                .slice(offset as usize, size as usize)
                .map(|bytes| {
                    Container::with_bytes(
                        bytes,
                        macho_endian_from_magic(unsafe {
                            core::ptr::read_unaligned(bytes.as_ptr() as *const mach_magic)
                        }),
                    )
                })
        })
        .find(|container| {
            container
                .deserialize_type_at_offset::<mach_header_64>(0)
                .map(|h| h.magic == mach_magic::MH_MAGIC_64 && h.cputype == CPU_TYPE_ARM64)
                .unwrap_or(false)
        })
        .map(|container| arm64_container_create(&container))
        .ok_or_else(|| "error: could not read `mach_header_64` from inner mach-o")?
}
