use futures::{executor::ThreadPool, StreamExt};
use wag::{
    async_handle_err,
    gui::{
        Background, Button, ButtonEvent, CellLimit, EventSource, LayerStack, Ribbon,
        RibbonOrientation, Root, SimpleButtonSkin, WBackground,
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
    let mut layer_stack = LayerStack::new(&compositor)?;
    let mut vribbon = Ribbon::new(compositor.clone(), RibbonOrientation::Vertical)?;
    let mut hribbon = Ribbon::new(compositor.clone(), RibbonOrientation::Horizontal)?;

    // let button_skin = DefaultButtonSkin::new(compositor.clone())?;
    let button_skin = SimpleButtonSkin::new(compositor.clone(), Colors::Magenta()?)?;
    let button = Button::new(&compositor, button_skin)?;

    let red_surface = Background::new(compositor.clone(), Colors::Red()?, true)?;
    let green_surface = Background::new(compositor.clone(), Colors::Green()?, true)?;
    let blue_surface = Background::new(compositor.clone(), Colors::Blue()?, true)?;

    async fn rotate_background_colors(
        a: &mut WBackground,
        b: &mut WBackground,
        c: &mut WBackground,
    ) -> wag::Result<()> {
        let a = a.upgrade();
        let b = b.upgrade();
        let c = c.upgrade();
        if let (Some(mut a), Some(mut b), Some(mut c)) = (a, b, c) {
            let ca = a.color();
            let cb = b.color();
            let cc = c.color();
            a.set_color(cb)?;
            b.set_color(cc)?;
            c.set_color(ca)?;
        }
        Ok(())
    }

    pool.spawn_ok(async_handle_err({
        let mut a = red_surface.downgrade();
        let mut b = green_surface.downgrade();
        let mut c = blue_surface.downgrade();
        let mut stream = button.event_stream();
        async move {
            // while let Some(event) = stream.next().await {
            //     if let PanelEvent::MouseInput { .. } = *event.as_ref() {
            //         rotate_background_colors(&mut a, &mut b, &mut c).await?;
            //     }
            while let Some(event) = stream.next().await {
                if ButtonEvent::Release(true) == *event.as_ref() {
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
    hribbon.add_panel(red_surface, CellLimit::default())?;
    hribbon.add_panel(green_surface, CellLimit::default())?;
    hribbon.add_panel(blue_surface, CellLimit::default())?;
    vribbon.add_panel(hribbon, CellLimit::new(4., 100., None, None))?;
    vribbon.add_panel(
        button,
        CellLimit::new(1., 50., Some(300.), Some(Vector2 { X: 0.5, Y: 0.8 })),
    )?;
    layer_stack.push_panel(vribbon)?;
    root.set_panel(layer_stack)?;

    let window = Window::new(compositor, "demo", root.visual(), root.tx_event_channel());
    let _window = window.open()?;
    run_message_loop();

    Ok(())
}
