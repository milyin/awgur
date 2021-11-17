pub mod gui;
pub mod window;

use futures::{task::SpawnError, Future};
use thiserror::Error;
use windows::core;

pub use winit::event::WindowEvent;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Bad element index")]
    BadIndex,
    #[error(transparent)]
    Spawn(SpawnError),
    #[error(transparent)]
    StdIO(std::io::Error),
    #[error(transparent)]
    AsyncObject(async_object::Error),
    #[error(transparent)]
    Windows(core::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<core::Error> for Error {
    fn from(e: core::Error) -> Self {
        Error::Windows(e)
    }
}

impl From<async_object::Error> for Error {
    fn from(e: async_object::Error) -> Self {
        Error::AsyncObject(e)
    }
}
impl From<SpawnError> for Error {
    fn from(e: SpawnError) -> Self {
        Error::Spawn(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::StdIO(e)
    }
}

pub fn unwrap_err(future: impl Future<Output = crate::Result<()>>) -> impl Future<Output = ()> {
    async { (future.await).unwrap() }
}
