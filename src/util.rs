/// Super unsafe, but useful when `'a` can be expressed as `'static`,
/// i.e. in a self referential struct.
pub unsafe fn as_static_mut<T>(item: &mut T) -> &'static mut T {
    std::mem::transmute(item)
}
