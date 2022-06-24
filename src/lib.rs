//! # WAG - Windows Asynchronous GUI
mod error;
pub mod gui;
pub mod window;

pub use error::{async_handle_err, Error, Result};
pub use winit::event::WindowEvent;
