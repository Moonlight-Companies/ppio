use std::convert::Infallible;

use futures::Future;

use crate::channel::{unbounded, Receiver, self};
use crate::pollers::{IntoPoller, Poller};
use crate::pushers::EmptyPusher;

pub trait State<T> {
    fn update(&mut self, state: T);
}

pub trait Poll {
    /// What you are sending to the rest of the app
    type Item: Send;

    /// Poll function
    fn poll(&mut self, tx: channel::Sender<Self::Item>) -> impl Future<Output = anyhow::Result<Infallible>> + Send + '_;
}

pub fn poll<P: Poll>(p: impl IntoPoller<P>) -> (Poller<P>, Receiver<P::Item>) {
    let (tx, rx) = unbounded();

    (Poller::new(p.into_poller(), tx), rx)
}

pub trait Push<T> {
    fn push(&mut self, item: T) -> impl Future<Output = anyhow::Result<()>> + Send + '_;
}

pub fn push<T>(rx: Receiver<T>) -> (EmptyPusher, Receiver<T>) {
    (EmptyPusher, rx)
}
