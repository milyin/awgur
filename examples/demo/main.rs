use futures::{executor::ThreadPool, StreamExt};
use wag::{
    async_handle_err,
    gui::{
        Background, BackgroundBuilder, CellLimit, LayerStack, Ribbon, RibbonOrientation, Root,
        SlotEventData, WBackground,
    },
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

    // let canvas_device = CanvasDevice::GetSharedDevice()?;
    // let composition_graphics_device =
    //     CanvasComposition::CreateCompositionGraphicsDevice(&compositor, &canvas_device)?;

    let root = Root::new(&pool, &compositor, Vector2 { X: 800., Y: 600. })?;
    let mut layer_stack = LayerStack::new(pool.clone(), &compositor, &mut root.slot())?;
    let layer = layer_stack.add_layer()?;

    let mut vribbon = Ribbon::new(
        pool.clone(),
        &compositor,
        layer,
        RibbonOrientation::Vertical,
    )?;

    let mut hribbon = Ribbon::new(
        pool.clone(),
        &compositor,
        vribbon.add_cell(CellLimit::new(4., 100., None, None))?,
        RibbonOrientation::Horizontal,
    )?;
    let mut button_slot = vribbon.add_cell(CellLimit::new(
        1.,
        50.,
        Some(300.),
        Some(Vector2 { X: 0.5, Y: 0.8 }),
    ))?;
    // let button = Background::new(
    //     pool.clone(),
    //     &compositor,
    //     button_slot.clone(),
    //     Colors::Pink()?,
    //     true,
    // )?;
    let button = BackgroundBuilder::builder()
        .color(Colors::Pink()?)
        .round_corners(true)
        .build()
        .new(pool.clone(), &compositor, &mut button_slot)?;
    let mut red_slot = hribbon.add_cell(CellLimit::default())?;
    let mut green_slot = hribbon.add_cell(CellLimit::default())?;
    let mut blue_slot = hribbon.add_cell(CellLimit::default())?;
    let red_surface = Background::new(
        pool.clone(),
        &compositor,
        &mut red_slot,
        Colors::Red()?,
        true,
    )?;
    let green_surface = Background::new(
        pool.clone(),
        &compositor,
        &mut green_slot,
        Colors::Green()?,
        true,
    )?;

    let blue_surface = Background::new(
        pool.clone(),
        &compositor,
        &mut blue_slot,
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

    pool.spawn_ok(async_handle_err({
        let mut a = red_surface.downgrade();
        let mut b = green_surface.downgrade();
        let mut c = blue_surface.downgrade();
        async move {
            let mut stream = button.slot().upgrade().unwrap().create_slot_event_stream();
            while let Some(event) = stream.next().await {
                if let SlotEventData::MouseInput = event.as_ref().data {
                    rotate_background_colors(&mut a, &mut b, &mut c).await?;
                }
            }
            Ok(())
        }
    }));

    // let window = Window::new(
    //     &compositor,
    //     "demo",
    //     800,
    //     600,
    //     root.visual(),
    //     root.tx_event_channel(),
    // )?;

    let window = Window::new(compositor, "demo", root.visual(), root.tx_event_channel());
    let _window = window.open()?;
    run_message_loop();

    Ok(())
}
