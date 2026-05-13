use crate::{
    container::{self, Container},
    libc, mmap,
};

pub fn disk_path_mmap<'dylib>(path: &[u8]) -> Result<Container<'dylib>, &'static str> {
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY, 0) }
        .map_err(|_| "could not open path")? as u32;

    let mut stat = libc::File::default();
    libc::fstat(fd as usize, &mut stat).map_err(|_| "could not stat cache file")?;
    let size = stat.st_size as usize;

    let bytes = unsafe {
        let ptr = mmap::mmap_file(
            size,
            mmap::PROT_READ | mmap::PROT_WRITE,
            mmap::MAP_PRIVATE,
            fd as i32,
            0,
        )
        .map_err(|e| "could not mmap cache file")?;
        core::slice::from_raw_parts(ptr.as_ptr(), size)
    };

    // fix: the endianness should be read from the header
    Ok(Container::with_bytes(bytes, container::Endian::Little))
}
