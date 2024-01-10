use std::convert::Infallible;

use futures::Future;

use crate::channel::{unbounded, Receiver, self};
use crate::pollers::{IntoPoller, Poller};
use crate::pushers::EmptyPusher;

// bikeshed
pub type PollOutput = anyhow::Result<Infallible>;
pub type PushOutput = anyhow::Result<()>;

pub trait State {
    type State;

    fn update(&mut self, state: Self::State);
}

pub trait Poll {
    /// What you are sending to the rest of the app
    type Item: Send;

    /// Poll function
    fn poll(&mut self, tx: channel::Sender<Self::Item>) -> impl Future<Output = PollOutput> + Send + '_;
}

pub fn poll<P: Poll>(p: impl IntoPoller<P>) -> (Poller<P>, Receiver<P::Item>) {
    let (tx, rx) = unbounded();

    (Poller::new(p.into(), tx), rx)
}

pub trait Push<T> {
    fn push(&mut self, item: T) -> impl Future<Output = PushOutput> + Send + '_;
}

pub fn push<T>(rx: Receiver<T>) -> (EmptyPusher, Receiver<T>) {
    (EmptyPusher, rx)
}
