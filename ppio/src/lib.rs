mod io;

mod pollers;
mod pushers;

mod deferred;
mod fut;

mod chan {
    pub use kanal::bounded_async as bounded;
    pub use kanal::unbounded_async as unbounded;
    pub use kanal::AsyncReceiver as Receiver;
    pub use kanal::AsyncSender as Sender;
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
    ($($fut:expr),* $(,)?) => {
        {
            tokio::select! {
                $(err = async move { $fut.await } => err),*
            }
        }
    };
}

#[macro_export]
macro_rules! allt {
    ($($fut:expr),* $(,)?) => {
        {
            tokio::select! {
                $(
                    err = tokio::spawn(async move { $fut.await }) => {
                        match err {
                            Ok(err) => err, 
                            Err(err) => Err(err.into()),
                        };
                    }
                ),*
            }
        }
    };
}
