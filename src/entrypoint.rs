use core::ptr::NonNull;

use alloc::vec::Vec;

use crate::{
    bindings_macho::{CPU_TYPE_ARM64, fat_arch, fat_header, mach_header_64, mach_magic},
    container::Container,
    dsc,
    dyld_shared_cache::DyldSharedCache,
    dynld::{dynamically_link, macho_endian_from_magic},
    fixups::Fixup,
    jump,
    libc::exit_error,
    println_err,
};

/// The main entrypoint of the dynld
///
/// This function is meant to take a pointer to the [`mach_header_64`] with
/// the size of the executable. It dynamically links it and executes it.
///
/// This function never returns and if an internal error happens, it will loop
/// infinitly.
pub fn dynld_entrypoint(
    _self_bind_fixups: Vec<Fixup>,
    executable_header: *mut mach_header_64,
    executable_size: usize,
    argc: usize,
    argv: *const *const u8,
    envp: *const *const u8,
    _apple: *const *const u8,
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

    // let mut image_infos = dsc::user64_dyld_all_image_infos::default();
    // if unsafe { dsc::dyld_all_image_infos_get(&mut image_infos) } == 0 {
    //     println_err!("failed to find the dyld_shared_cache base address");
    //     exit_error()
    // }

    // print!("{image_infos:#?}");

    let mut dyld_shared_cache = DyldSharedCache::new();
    if let Err(err) = unsafe {
        dyld_shared_cache.from_live_mapping_initialize(
            dyld_shared_cache_baddr_in_vm as *const u8,
            dyld_shared_cache_baddr_in_vm - 0x180000000,
        )
    } {
        println_err!("failed initializing dyld shared cache: '{err}'");
        exit_error()
    }

    let entry_point = dynamically_link(&dyld_shared_cache, image).unwrap_or_else(|err| {
        println_err!("dylinking error: '{err}', giving up...");
        exit_error()
    });

    // we only check that the entrypoint is not NULL, if
    // it is something else, we expect it to be valid at this point
    if entry_point == 0 {
        println_err!("the entry_point of the program is NULL, giving up...");
        exit_error()
    }

    //unsafe {
    //    tlv_initialize_descriptors_export(executable_header as *const mach_header_64);
    //}

    // jump to the entrypoint. We should make sure that this is OK to do
    // before doing this, we fully expect everything to work
    unsafe {
        jump::entry_and_ret(
            NonNull::new_unchecked(entry_point as *mut u8),
            argc,
            argv,
            envp,
        )
    };

    // THIS FUNCTION CANNOT RETURN! the caller nulled the link register
    // and the frame pointer, we try to exit but trigger and inifite loop
    // if that does not work

    exit_error()
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
