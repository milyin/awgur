mod interop;
mod native_window;
mod wide_string;

pub mod native {
    pub use super::native_window::Window;
}

pub use interop::create_dispatcher_queue_controller;
pub use interop::create_dispatcher_queue_controller_for_current_thread;
use windows::System::DispatcherQueueController;
use windows::Win32::System::WinRT::RoInitialize;
use windows::Win32::System::WinRT::RoUninitialize;
use windows::Win32::System::WinRT::RO_INIT_MULTITHREADED;

pub struct WindowThread {
    pub controller: DispatcherQueueController,
}

impl Drop for WindowThread {
    fn drop(&mut self) {
        unsafe { RoUninitialize() }
    }
}

pub fn initialize_window_thread() -> crate::Result<WindowThread> {
    unsafe { RoInitialize(RO_INIT_MULTITHREADED)? }
    Ok(WindowThread {
        controller: create_dispatcher_queue_controller_for_current_thread()?,
    })
}
