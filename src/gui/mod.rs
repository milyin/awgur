mod background;
mod frame;
mod ribbon;
mod slot;
// mod text;

pub use background::{Background, BackgroundKeeper, BackgroundTag};
pub use frame::{Frame, KFrame, TFrame};
pub use ribbon::{CellLimit, Ribbon, RibbonKeeper, RibbonOrientation, RibbonTag};
pub use slot::{Slot, SlotKeeper, SlotPlug, SlotTag};
// pub use text::{Text, TextKeeper, TextTag};