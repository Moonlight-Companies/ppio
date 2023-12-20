use std::mem::MaybeUninit;
use std::pin::Pin;

pub struct Later<T>(MaybeUninit<T>);

impl<T> Later<T> {
    pub unsafe fn new() -> Self {
        Self(MaybeUninit::uninit())
    }

    pub fn set(this: &mut Later<T>, val: T) {
        this.0.write(val);
    }

    pub fn get(this: &Later<T>) -> &T {
        unsafe { &*this.0.as_ptr() }
    }

    pub fn get_mut(this: &mut Later<T>) -> &mut T {
        unsafe { &mut *this.0.as_mut_ptr() }
    }

    pub fn get_pin_mut(this: Pin<&mut Later<T>>) -> Pin<&mut T> {
        unsafe { this.map_unchecked_mut(|this| &mut *this.0.as_mut_ptr()) }
    }

    pub fn take(this: Later<T>) -> T {
        let t = unsafe { std::ptr::read(this.0.assume_init_ref()) };
        std::mem::forget(this);
        t
    }
}

impl<T> Drop for Later<T> {
    fn drop(&mut self) {
        unsafe { self.0.assume_init_drop() }
    }
}

impl<T> std::ops::Deref for Later<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        Self::get(self)
    }
}

impl<T> std::ops::DerefMut for Later<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::get_mut(self)
    }
}
