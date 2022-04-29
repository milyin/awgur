use futures::{executor::ThreadPool, StreamExt};
use wag::{
    async_handle_err,
    gui::{
        Background, CellLimit, LayerStack, Panel, PanelEventData, Ribbon, RibbonOrientation, Root,
        WBackground,
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

    let mut root = Root::new(&pool, &compositor, Vector2 { X: 800., Y: 600. })?;
    let mut layer_stack = LayerStack::new(compositor.clone())?;
    let mut vribbon = Ribbon::new(compositor.clone(), RibbonOrientation::Vertical)?;
    let mut hribbon = Ribbon::new(compositor.clone(), RibbonOrientation::Horizontal)?;

    let button = Background::new(compositor.clone(), Colors::Pink()?, true)?;
    // let button = Button::new(pool.clone(), &compositor, &mut button_slot)?;
    // let button_skin = ButtonSkin::new(
    //     pool.clone(),
    //     &compositor,
    //     &mut button.slot(),
    //     button.create_button_event_stream(),
    // )?;

    let red_surface = Background::new(compositor.clone(), Colors::Red()?, true)?;
    let green_surface = Background::new(compositor.clone(), Colors::Green()?, true)?;
    let blue_surface = Background::new(compositor.clone(), Colors::Blue()?, true)?;

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
        let mut stream = button.panel_event_stream();
        async move {
            while let Some(event) = stream.next().await {
                if let PanelEventData::MouseInput { .. } = event.as_ref().data {
                    rotate_background_colors(&mut a, &mut b, &mut c).await?;
                }
                // if ButtonEventData::Release(true) == event.as_ref().data {
                //     rotate_background_colors(&mut a, &mut b, &mut c).await?;
                // }
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
    hribbon.add_cell(red_surface, CellLimit::default())?;
    hribbon.add_cell(green_surface, CellLimit::default())?;
    hribbon.add_cell(blue_surface, CellLimit::default())?;
    vribbon.add_cell(hribbon, CellLimit::new(4., 100., None, None))?;
    vribbon.add_cell(
        button,
        CellLimit::new(1., 50., Some(300.), Some(Vector2 { X: 0.5, Y: 0.8 })),
    )?;
    layer_stack.add_layer(vribbon)?;
    root.set_panel(layer_stack)?;

    let window = Window::new(compositor, "demo", root.visual(), root.tx_event_channel());
    let _window = window.open()?;
    run_message_loop();

    Ok(())
}
