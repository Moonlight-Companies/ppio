use std::{future::IntoFuture, pin::Pin};

use futures::future::BoxFuture;
use pin_project_lite::pin_project;

use crate::chan::{bounded, Receiver, Sender};
use crate::deferred::{Defer, DeferFuture};
use crate::io::{Poll, PollOutput};

pin_project! {
    pub struct Poller<P: Poll> {
        poller: P,
        rx: Option<Receiver<P::Item>>,
        tx: Vec<Sender<P::Item>>,
    }
}

impl<P: Poll> Poller<P> {
    pub(crate) fn new(poller: P, tx: Vec<Sender<P::Item>>) -> Self {
        Self {
            poller,
            rx: None,
            tx,
        }
    }
}

pin_project! {
    pub struct Fut<'a, P: Poll> {
        #[pin]
        fut: BoxFuture<'a, PollOutput>,
        rx: Receiver<P::Item>,
        tx: Vec<Sender<P::Item>>,
    }
}

impl<'a, P: Poll> std::future::Future for Fut<'a, P>
where
    P::Item: Clone,
{
    type Output = PollOutput;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut proj = self.project();

        let res = proj.fut.as_mut().poll(cx);

        if let Ok(Some(item)) = proj.rx.try_recv_realtime() {
            proj.tx.iter().for_each(|tx| {
                drop(tx.try_send(item.clone()));
            });
        }

        res
    }
}

impl<P: Poll + 'static> IntoFuture for Poller<P>
where
    P::Item: Clone,
{
    type Output = PollOutput;
    type IntoFuture = DeferFuture<'static, Self>;

    fn into_future(self) -> Self::IntoFuture {
        DeferFuture::new(self)
    }
}

impl<'a, P: Poll + 'a> Defer<'a> for Poller<P>
where
    P::Item: Clone,
{
    type Future = Fut<'a, P>;

    fn into_fut(self: Pin<&'a mut Self>) -> Self::Future {
        let proj = self.project();

        let (tx, rx) = bounded(0);
        proj.rx.replace(rx);

        Fut {
            fut: proj.poller.poll(tx),
            rx: proj.rx.take().unwrap(),
            tx: std::mem::take(proj.tx),
        }
    }
}
