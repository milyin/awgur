use futures::executor::ThreadPool;
use wag::{
    gui::{BackgroundKeeper, CellLimit, KLayerStack, KRibbon, RibbonOrientation},
    window::{initialize_window_thread, native::Window},
};
use windows::UI::{Colors, Composition::Compositor};

fn main() -> wag::Result<()> {
    let _window_thread = initialize_window_thread()?;
    let pool = ThreadPool::new()?;
    let compositor = Compositor::new()?;
    let window = Window::new(&compositor, "demo", 800, 600)?;
    let layer_stack = KLayerStack::new(pool.clone(), &compositor, &window.slot())?;
    let layer = layer_stack.tag().add_layer()?;
    let ribbon = KRibbon::new(
        pool.clone(),
        &compositor,
        layer,
        RibbonOrientation::Horizontal,
    )?;
    let _red_surface = BackgroundKeeper::new(
        pool.clone(),
        &compositor,
        ribbon.tag().add_cell(CellLimit::default())?,
        Colors::Red()?,
        true,
    )?;
    let _green_surface = BackgroundKeeper::new(
        pool.clone(),
        &compositor,
        ribbon.tag().add_cell(CellLimit::default())?,
        Colors::Green()?,
        true,
    )?;

    let _blue_surface = BackgroundKeeper::new(
        pool.clone(),
        &compositor,
        ribbon.tag().add_cell(CellLimit::default())?,
        Colors::Blue()?,
        true,
    )?;

    Window::run();

    Ok(())
}
