use crate::channel::Receiver;
use crate::io::Push;

pub use basic::Pusher;
pub use function::Pusher as FunctionPusher;
pub use stateful::Pusher as StatefulPusher;

mod basic;
mod function;
mod stateful;

pub trait IntoPusher<P> {
    fn into(self) -> P;
}

impl<P> IntoPusher<P> for P {
    fn into(self) -> P {
        self
    }
}

pub struct EmptyPusher;

pub trait To<T> {
    fn to<P: Push<T>>(self, p: P) -> Pusher<T, P>;
    fn to_fn<F: Fn(T)>(self, p: F) -> FunctionPusher<T, F>;
}

impl<T> To<T> for (EmptyPusher, Receiver<T>) {
    fn to<P: Push<T>>(self, p: P) -> Pusher<T, P> {
        Pusher::new(p, self.1)
    }
    fn to_fn<F: Fn(T)>(self, p: F) -> FunctionPusher<T, F> {
        FunctionPusher::new(p, self.1)
    }
}

pub trait UpgradePusher<T, P: Push<T>> {
    fn with_state<S>(self, state_rx: Receiver<S>) -> StatefulPusher<T, S, P>;
}

impl<T, P: Push<T>> UpgradePusher<T, P> for Pusher<T, P> {
    fn with_state<S>(self, state_rx: Receiver<S>) -> StatefulPusher<T, S, P> {
        let (tx, pusher) = self.take_parts();
        StatefulPusher::new(pusher, tx, state_rx)
    }
}