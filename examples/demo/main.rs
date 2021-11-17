use futures::executor::ThreadPool;
use wag::{
    gui::{Background, CellLimit, LayerStack, Ribbon, RibbonOrientation},
    window::{
        initialize_window_thread,
        native::{run_message_loop, Window},
    },
};
use windows::UI::{Colors, Composition::Compositor};

fn main() -> wag::Result<()> {
    let _window_thread = initialize_window_thread()?;
    let pool = ThreadPool::new()?;
    let compositor = Compositor::new()?;
    let window = Window::new(&compositor, "demo", 800, 600)?;
    let mut layer_stack = LayerStack::new(pool.clone(), &compositor, &window.slot())?;
    let layer = layer_stack.add_layer()?;
    let mut ribbon = Ribbon::new(
        pool.clone(),
        &compositor,
        layer,
        RibbonOrientation::Horizontal,
    )?;
    let _red_surface = Background::new(
        pool.clone(),
        &compositor,
        ribbon.add_cell(CellLimit::default())?,
        Colors::Red()?,
        true,
    )?;
    let _green_surface = Background::new(
        pool.clone(),
        &compositor,
        ribbon.add_cell(CellLimit::default())?,
        Colors::Green()?,
        true,
    )?;

    let _blue_surface = Background::new(
        pool.clone(),
        &compositor,
        ribbon.add_cell(CellLimit::default())?,
        Colors::Blue()?,
        true,
    )?;

    run_message_loop();

    Ok(())
}
