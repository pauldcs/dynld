use core::ptr::NonNull;

#[derive(Debug)]
pub enum DynamicLib<'path> {
    Cached(&'path str),
    Loaded(&'path str, NonNull<u8>),
}

impl<'path> DynamicLib<'path> {
    pub fn new_cached(path: &'path str) -> Self {
        Self::Cached(path)
    }
}

impl<'path> DynamicLib<'path> {
    pub fn load(&mut self) {
        unimplemented!()
    }

    pub fn unload(&mut self) {
        unimplemented!()
    }
}

impl<'path> Drop for DynamicLib<'path> {
    fn drop(&mut self) {
        if let DynamicLib::Loaded(..) = self {
            self.unload();
        }
    }
}
