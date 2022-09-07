//! # WAG - Windows Asynchronous GUI
mod error;
pub mod gui;
pub mod window;

pub use error::{handle_err, on_err, Error, Result};
pub use winit::event::WindowEvent;
