use crate::chan::Receiver;
use crate::io::{Push, PushOutput};

pub use basic::Pusher;
pub use function::Pusher as FunctionPusher;

mod function;

mod basic;

pub trait IntoPusher<P> {
    fn into(self) -> P;
}

impl<P> IntoPusher<P> for P {
    fn into(self) -> P {
        self
    }
}

impl<T: Send, U> Push for fn(T) -> U {
    type Item = T;

    async fn push(&mut self, item: Self::Item) -> PushOutput {
        self(item);

        Ok(())
    }
}

pub struct EmptyPusher;

pub trait To<T> {
    fn to<P: Push<Item = T>>(self, p: P) -> Pusher<P>;
    fn to_fn<F: Fn(T)>(self, p: F) -> FunctionPusher<T, F>;
}

impl<T> To<T> for (EmptyPusher, Receiver<T>) {
    fn to<P: Push<Item = T>>(self, p: P) -> Pusher<P> {
        Pusher::new(p, self.1)
    }
    fn to_fn<F: Fn(T)>(self, p: F) -> FunctionPusher<T, F> {
        FunctionPusher::new(p, self.1)
    }
}
