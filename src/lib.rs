mod io;

mod pollers;
mod pushers;

mod util;

pub mod channel {
    pub use async_channel::{
        bounded, unbounded, Receiver, Recv, RecvError, Send, SendError, Sender,
    };
}

pub mod prelude {
    pub use crate::io::*;
    pub use crate::pollers::*;
    pub use crate::pushers::*;

    pub use std::convert::Infallible; 
    pub use anyhow;

    pub use crate::{all, allt};
}

/// wraps anyhow::Error to allow the macro to determine whether the error should bubble up
#[derive(Debug)]
pub enum Error {
    Internal(anyhow::Error),
    User(anyhow::Error),
}

impl Error {
    pub fn inner(self) -> anyhow::Error {
        match self {
            Self::Internal(e) => e,
            Self::User(e) => e,
        }
    }
}

pub mod macro_helpers {
    use std::future::Future;
    use std::convert::Infallible;

    use futures::FutureExt;
    
    /// reexport for proc macro
    pub use tokio;

    pub fn internal_spawn<F>(fut: F) -> impl Future<Output = Result<Infallible, crate::Error>>
    where
        F: std::future::Future<Output = Result<Infallible, crate::Error>> + Send + 'static,
    {
        let jh = tokio::spawn(fut);

        jh.map(|res| match res {
            Ok(Err(e)) => Err(e),
            Err(e) => Err(crate::Error::Internal(e.into())),
            Ok(Ok(impossible)) => Ok(impossible) 
        })
    } 

    #[macro_export]
    macro_rules! all {
        ($($fut:expr),+ $(,)?) => {
            async move {
                $crate::macro_helpers::tokio::select! {
                    biased;
                    $(Err($crate::Error::User(err)) = async move { $fut.await } => Result::<Infallible, _>::Err($crate::Error::User(err))),+
                    , else => Err($crate::Error::Internal(anyhow::anyhow!("no user errors reported, but all tasks failed")))
                }
            }
        };
    }

    #[macro_export]
    macro_rules! allt {
        ($($fut:expr),+ $(,)?) => {
            async move {
                $crate::macro_helpers::tokio::select! {
                    biased;
                    $(Err($crate::Error::User(err)) = $crate::macro_helpers::internal_spawn(async move { $fut.await }) => Result::<Infallible, _>::Err($crate::Error::User(err))),+
                    , else => Err($crate::Error::Internal(anyhow::anyhow!("no user errors reported, but all tasks failed")))
                }
            };
        };
    }
}
