use awgur::{
    gui::{RibbonKeeper, RibbonOrientation},
    window::{initialize_window_thread, native::Window},
};
use futures::executor::ThreadPool;

fn main() -> awgur::Result<()> {
    let _window_thread = initialize_window_thread()?;
    let pool = ThreadPool::new()?;
    let window = Window::new("demo", 800, 600)?;
    let q = window.root_visual().Size()?;
    let compositor = window.compositor();
    let _frame = RibbonKeeper::new(
        pool.clone(),
        compositor.clone(),
        window.slot(),
        RibbonOrientation::Stack,
    )?;

    Ok(())
}
