use futures::{executor::ThreadPool, StreamExt};
use wag::{
    gui::{Background, CellLimit, LayerStack, Ribbon, RibbonOrientation, WBackground},
    unwrap_err,
    window::{
        initialize_window_thread,
        native::{run_message_loop, Window},
    },
};
use windows::{
    Foundation::Numerics::Vector2,
    UI::{Colors, Composition::Compositor},
};

fn main() -> wag::Result<()> {
    let _window_thread = initialize_window_thread()?;
    let pool = ThreadPool::builder() //.pool_size(8)
        .create()?;
    let compositor = Compositor::new()?;
    let window = Window::new(pool.clone(), &compositor, "demo", 800, 600)?;
    let mut layer_stack = LayerStack::new(pool.clone(), &compositor, &mut window.slot())?;
    let layer = layer_stack.add_layer(pool.clone())?;

    let mut vribbon = Ribbon::new(
        pool.clone(),
        &compositor,
        layer,
        RibbonOrientation::Vertical,
    )?;

    let mut hribbon = Ribbon::new(
        pool.clone(),
        &compositor,
        vribbon.add_cell(pool.clone(), CellLimit::new(4., 100., None, None))?,
        RibbonOrientation::Horizontal,
    )?;
    let button_slot = vribbon.add_cell(
        pool.clone(),
        CellLimit::new(1., 50., Some(300.), Some(Vector2 { X: 0.5, Y: 0.8 })),
    )?;
    let _button = Background::new(
        pool.clone(),
        &compositor,
        button_slot.clone(),
        Colors::Pink()?,
        true,
    )?;
    let red_slot = hribbon.add_cell(pool.clone(), CellLimit::default())?;
    let green_slot = hribbon.add_cell(pool.clone(), CellLimit::default())?;
    let blue_slot = hribbon.add_cell(pool.clone(), CellLimit::default())?;
    let red_surface = Background::new(
        pool.clone(),
        &compositor,
        red_slot.clone(),
        Colors::Red()?,
        true,
    )?;
    let green_surface = Background::new(
        pool.clone(),
        &compositor,
        green_slot.clone(),
        Colors::Green()?,
        true,
    )?;

    let blue_surface = Background::new(
        pool.clone(),
        &compositor,
        blue_slot.clone(),
        Colors::Blue()?,
        true,
    )?;

    async fn rotate_background_colors(
        a: &mut WBackground,
        b: &mut WBackground,
        c: &mut WBackground,
    ) -> wag::Result<()> {
        let ca = a.async_color().await;
        let cb = b.async_color().await;
        let cc = c.async_color().await;
        if let (Some(ca), Some(cb), Some(cc)) = (ca, cb, cc) {
            let _ = a.async_set_color(cb).await?;
            let _ = b.async_set_color(cc).await?;
            let _ = c.async_set_color(ca).await?;
        }
        Ok(())
    }

    pool.spawn_ok(unwrap_err({
        let mut a = red_surface.downgrade();
        let mut b = green_surface.downgrade();
        let mut c = blue_surface.downgrade();
        async move {
            while let Some(_) = button_slot.on_slot_mouse_input().next().await {
                rotate_background_colors(&mut a, &mut b, &mut c).await?;
            }
            Ok(())
        }
    }));

    run_message_loop();

    Ok(())
}
