mod background;
mod frame;
mod ribbon;
mod slot;
// mod text;
mod slot_event;

pub use background::{Background, BackgroundKeeper, BackgroundTag};
pub use frame::{Frame, FrameKeeper, FrameTag};
pub use ribbon::{CellLimit, Ribbon, RibbonKeeper, RibbonOrientation, RibbonTag};
pub use slot::{Slot, SlotKeeper, SlotTag};
// pub use text::{Text, TextKeeper, TextTag};
pub use slot_event::{
    MouseLeftPressed, MouseLeftPressedFocused, ReceiveSlotEvent, SendSlotEvent, SlotSize,
};

use futures::task::SpawnError;
// pub use ribbon::{Ribbon, RibbonKeeper, RibbonTag};
use thiserror::Error;
use windows::runtime;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Bad element index")]
    BadIndex,
    #[error(transparent)]
    Spawn(SpawnError),
    #[error(transparent)]
    AsyncObject(async_object::Error),
    #[error(transparent)]
    Windows(runtime::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<runtime::Error> for Error {
    fn from(e: runtime::Error) -> Self {
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
