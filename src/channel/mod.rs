mod maprecv {
    use std::future::Future;

    use futures::stream::Map;
    use futures::{FutureExt, StreamExt};
    use kanal::ReceiveError;

    pub trait MapFn<F: Send, I>: Fn(F) -> I + Send + Sync + 'static {}

    impl<F: Send, I, T: Fn(F) -> I + Send + Sync + 'static> MapFn<F, I> for T {}

    pub type ReceiveStream<'a, T, R> = Map<kanal::ReceiveStream<'a, R>, &'a dyn MapFn<R, T>>;

    pub struct Receiver<T, R = T>(kanal::AsyncReceiver<R>, Box<dyn MapFn<R, T>>);

    impl<T: Send, R: Send> Receiver<T, R> {
        pub fn recv(&self) -> impl Future<Output = Result<T, ReceiveError>> + '_ {
            self.0.recv().map(|r| r.map(self.1.as_ref()))
        }

        pub fn stream(&self) -> ReceiveStream<'_, T, R> {
            self.0.stream().map(self.1.as_ref())
        }

        pub fn map<N>(self, f: impl MapFn<R, N>) -> Receiver<N, R> {
            Receiver(self.0, Box::new(f))
        }
    }

    impl<T> Receiver<T, T> {
        pub fn inner(self) -> kanal::AsyncReceiver<T> {
            self.0
        }
    }

    impl<T: Send> From<kanal::AsyncReceiver<T>> for Receiver<T> {
        fn from(value: kanal::AsyncReceiver<T>) -> Self {
            Receiver(value, Box::new(|t| t))
        }
    }
}

pub use kanal::AsyncSender as Sender;
pub use maprecv::{MapFn, ReceiveStream, Receiver};

pub fn bounded<T: Send>(size: usize) -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = kanal::bounded_async(size);

    (tx, rx.into())
}

pub fn unbounded<T: Send>() -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = kanal::unbounded_async();

    (tx, rx.into())
}
