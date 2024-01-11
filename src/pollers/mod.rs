use crate::channel::{bounded, unbounded, Receiver};
use crate::io::{Poll, State};

pub use basic::Poller;
pub use broadcast::Poller as BroadcastPoller;
pub use stateful::Poller as StatefulPoller;

mod basic;

mod broadcast;

mod stateful;

pub trait IntoPoller<P: Poll> {
    fn into(self) -> P;
}

impl<P: Poll> IntoPoller<P> for P {
    fn into(self) -> P {
        self
    }
}

pub trait UpgradePoller<P: Poll> {
    fn broadcast<const C: usize>(self) -> (BroadcastPoller<P>, [Receiver<P::Item>; C])
    where
        P::Item: Clone;

    fn with_state<S>(self, state_rx: Receiver<S>) -> (StatefulPoller<S, P>, Receiver<P::Item>)
    where
        P: State<S>;
}

impl<P: Poll> UpgradePoller<P> for (Poller<P>, Receiver<P::Item>) {
    fn broadcast<const C: usize>(self) -> (BroadcastPoller<P>, [Receiver<P::Item>; C])
    where
        P::Item: Clone,
    {
        let p = self.0.take_poller();

        let mut txs = Vec::with_capacity(C);
        let rxs = core::array::from_fn(|_| {
            let (tx, rx) = bounded(10);
            txs.push(tx);
            rx
        });

        (BroadcastPoller::new(p, txs), rxs)
    }

    fn with_state<S>(self, state_rx: Receiver<S>) -> (StatefulPoller<S, P>, Receiver<P::Item>)
    where
        P: State<S>,
    {
        let p = self.0.take_poller();

        let (tx, rx) = unbounded();

        (StatefulPoller::new(p, tx, state_rx), rx)
    }
}
