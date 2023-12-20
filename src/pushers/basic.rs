use std::convert::Infallible;
use std::future::IntoFuture;
use std::{future::Future, pin::Pin, task};

use futures::future::BoxFuture;
use kanal::ReceiveFuture;
use pin_project_lite::pin_project;

use crate::chan::Receiver;
use crate::fut::{extend, Either};
use crate::io::{PollOutput, Push, PushOutput};

pub struct Pusher<P: Push> {
    rx: Receiver<P::Item>,
    pusher: P,
}

impl<P: Push> Pusher<P> {
    pub fn new(pusher: P, rx: Receiver<P::Item>) -> Self {
        Self { rx, pusher }
    }
}

impl<P: Push + 'static> IntoFuture for Pusher<P> {
    type Output = PollOutput;
    type IntoFuture = Fut<'static, P>;

    fn into_future(self) -> Self::IntoFuture {
        Fut {
            inner: None,
            rx: self.rx,
            pusher: self.pusher,
        }
    }
}

type FutFut<'a, T> = Either<ReceiveFuture<'a, T>, BoxFuture<'a, PushOutput>>;

pin_project! {
    pub struct Fut<'a, P: Push> {
        #[pin]
        inner: Option<FutFut<'a, P::Item>>,
        rx: Receiver<P::Item>,
        pusher: P,
    }
}

impl<'a, P: Push + 'a> Future for Fut<'a, P> {
    type Output = Result<Infallible, anyhow::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let mut proj = self.project();

        loop {
            let res = match proj.inner.as_mut().as_pin_mut() {
                // poll the inner future
                Some(mut inner) => futures::ready!(inner.as_mut().poll(cx)),
                // simulate getting Ok(()) from the pusher, so we set up recv-ing
                None => Either::B(Ok(())),
            };

            match res {
                Either::A(Ok(item)) => {
                    // safety: this is safe since we follow the aliasing rules
                    //         and because the lifetime is effectively 'static ('a)
                    let fut = unsafe { extend(proj.pusher).push(item) };
                    proj.inner.set(Some(Either::B(fut)));
                }
                Either::B(Ok(())) => {
                    // safety: same as above
                    let fut = unsafe { extend(proj.rx).recv() };
                    proj.inner.set(Some(Either::A(fut)));
                }
                Either::A(Err(err)) => return task::Poll::Ready(Err(err.into())),
                Either::B(Err(err)) => return task::Poll::Ready(Err(err)),
            };
        }
    }
}
