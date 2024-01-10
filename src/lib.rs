mod io;

mod pollers;
mod pushers;

mod util;

mod chan {
    pub use async_channel::{
        bounded, unbounded, Receiver, Recv, RecvError, Send, SendError, Sender,
    };
}

pub mod prelude {
    pub use crate::chan::*;
    pub use crate::io::*;
    pub use crate::pollers::*;
    pub use crate::pushers::*;

    pub use crate::{all, allt};
}

/// reexport for proc macro
pub use tokio;

#[macro_export]
macro_rules! all {
    ($($fut:expr),+ $(,)?) => {
        async move {
            $crate::tokio::select! {
                biased;
                $(err = async move { $fut.await } => err),+
            }
        }
    };
}

#[macro_export]
macro_rules! allt {
    ($($fut:expr),+ $(,)?) => {
        async move {
            $crate::tokio::select! {
                biased;
                $(
                    err = tokio::spawn(async move { $fut.await }) => {
                        match err {
                            Ok(err) => err,
                            Err(err) => Err(anyhow::Error::from(err)),
                        }
                    }
                ),+
            }
        };
    };
}
