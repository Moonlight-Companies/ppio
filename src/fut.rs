use std::{future::Future, pin::Pin, task};

pub enum Either<A, B> {
    A(A),
    B(B),
}

impl<A, B> Either<A, B> {
    #[inline(always)]
    pub fn as_pin_mut(self: Pin<&mut Self>) -> Either<Pin<&mut A>, Pin<&mut B>> {
        // safety: see Option::as_pin_mut
        unsafe {
            match Pin::get_unchecked_mut(self) {
                Self::A(a) => Either::A(Pin::new_unchecked(a)),
                Self::B(b) => Either::B(Pin::new_unchecked(b)),
            }
        }
    }
}

impl<A: Future, B: Future> Future for Either<A, B> {
    type Output = Either<A::Output, B::Output>;

    #[inline(always)]
    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        match self.as_pin_mut() {
            Either::A(a) => a.poll(cx).map(Either::A),
            Either::B(b) => b.poll(cx).map(Either::B),
        }
    }
}

pub unsafe fn extend<'a, T>(item: &mut T) -> &'a mut T {
    std::mem::transmute(item)
}
