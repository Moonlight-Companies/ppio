use std::{future::Future, pin::Pin, task};

use pin_project_lite::pin_project;





pub trait Defer<'s>: 's {
    type Future: Future + 's;

    fn into_fut(self: Pin<&'s mut Self>) -> Self::Future;
}

pin_project! {
    pub struct DeferFuture<'a, T: Defer<'a>> {
        #[pin]
        fut: Option<T::Future>,
        #[pin]
        defer: T,
    }
}

impl<'a, T: Defer<'a>> DeferFuture<'a, T> {
    pub fn new(defer: T) -> Self {
        Self { fut: None, defer }
    }
}

impl<'a, T: Defer<'a>> Future for DeferFuture<'a, T> {
    type Output = <T::Future as Future>::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let mut proj = self.project();

        if proj.fut.is_none() {
            // transmute into SOME LIFETIME
            // safety: the ref is valid for the lifetime of this, so it's effectively static
            // https://users.rust-lang.org/t/saving-variables-interwoven-by-reference-in-one-structure/55746/12
            let defer: Pin<&'a mut T> = unsafe { std::mem::transmute(proj.defer.as_mut()) };

            proj.fut.set(Some(defer.into_fut()));
        }

        proj.fut.as_pin_mut().unwrap().poll(cx)
    }
}






pub trait Undefer<'a>: Defer<'a> {
    type State;

    fn undefer(fut: Pin<&mut Self::Future>) -> Option<Self::State>;
    fn update(self: Pin<&mut Self>, state: Self::State);
}

pin_project! {
    pub struct UndeferFuture<'a, T: Undefer<'a>> {
        #[pin]
        fut: Option<T::Future>,
        #[pin]
        defer: T,
    }
}

impl<'a, T: Undefer<'a>> UndeferFuture<'a, T> {
    pub fn new(defer: T) -> Self {
        Self { fut: None, defer }
    }
}

impl<'a, T: Undefer<'a>> Future for UndeferFuture<'a, T> {
    type Output = <T::Future as Future>::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let mut proj = self.project();

        let state = proj.fut.as_mut().as_pin_mut().and_then(T::undefer);

        if proj.fut.is_none() || state.is_some() {
            // drop the old future
            proj.fut.set(None);

            if let Some(state) = state {
                T::update(proj.defer.as_mut(), state);
            }

            // transmute into SOME LIFETIME
            // safety: self referential structs are valid for 'static according to:
            // https://users.rust-lang.org/t/saving-variables-interwoven-by-reference-in-one-structure/55746/12
            let defer: Pin<&'a mut T> = unsafe { std::mem::transmute(proj.defer.as_mut()) };

            proj.fut.set(Some(defer.into_fut()));
        }

        proj.fut.as_pin_mut().unwrap().poll(cx)
    }
}
