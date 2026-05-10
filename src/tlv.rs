use crate::bindings_macho::mach_header_64;

unsafe extern "C" {
    pub fn tlv_initialize_descriptors_export(mh: *const mach_header_64);
}
