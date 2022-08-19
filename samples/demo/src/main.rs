use std::sync::{Arc, Weak};

use futures::{executor::ThreadPool, StreamExt};
use wag::{
    async_handle_err,
    gui::{
        spawn_window_event_receiver, Background, BackgroundParams, Button, ButtonEvent,
        ButtonParams, CellLimit, EventSource, LayerStack, LayerStackParams, Ribbon,
        RibbonOrientation, RibbonParams, SimpleButtonSkin, SimpleButtonSkinParams,
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

// use ::windows_app::Microsoft::Windows::System::Power::*;

fn main() -> wag::Result<()> {
    // ::windows_app::bootstrap::initialize()?;
    // let charge = PowerManager::RemainingChargePercent()?;
    // println!("Remaining charge: {charge}%");

    let _window_thread = initialize_window_thread()?;
    let pool = ThreadPool::builder().pool_size(8).create()?;
    let compositor = Compositor::new()?;

    // let canvas_device = CanvasDevice::GetSharedDevice()?;
    // let composition_graphics_device =
    //     CanvasComposition::CreateCompositionGraphicsDevice(&compositor, &canvas_device)?;

    let b = || -> wag::Result<Arc<Button>> {
        let button_skin: Arc<SimpleButtonSkin> = SimpleButtonSkinParams::builder()
            .compositor(compositor.clone())
            .color(Colors::Magenta()?)
            .text("Rotate".to_owned())
            .spawner(pool.clone())
            .build()
            .try_into()?;
        let button = ButtonParams::builder()
            .skin(button_skin)
            .compositor(compositor.clone())
            .build()
            .try_into()?;
        Ok(button)
    };

    let button = b()?;

    let red_surface = BackgroundParams::builder()
        .compositor(compositor.clone())
        .color(Colors::Red()?)
        .round_corners(true)
        .build()
        .try_into()?;
    let green_surface = BackgroundParams::builder()
        .compositor(compositor.clone())
        .color(Colors::Green()?)
        .round_corners(true)
        .build()
        .try_into()?;
    let blue_surface = BackgroundParams::builder()
        .compositor(compositor.clone())
        .color(Colors::Blue()?)
        .round_corners(true)
        .build()
        .try_into()?;

    async fn rotate_background_colors(
        a: &Weak<Background>,
        b: &Weak<Background>,
        c: &Weak<Background>,
    ) -> wag::Result<()> {
        let a = a.upgrade();
        let b = b.upgrade();
        let c = c.upgrade();
        if let (Some(a), Some(b), Some(c)) = (a, b, c) {
            let ca = a.color().await;
            let cb = b.color().await;
            let cc = c.color().await;
            a.set_color(cb).await?;
            b.set_color(cc).await?;
            c.set_color(ca).await?;
        }
        Ok(())
    }

    pool.spawn_ok(async_handle_err({
        let a = Arc::downgrade(&red_surface);
        let b = Arc::downgrade(&green_surface);
        let c = Arc::downgrade(&blue_surface);
        let mut stream = button.event_stream();
        async move {
            // while let Some(event) = stream.next().await {
            //     if let PanelEvent::MouseInput { .. } = *event.as_ref() {
            //         rotate_background_colors(&mut a, &mut b, &mut c).await?;
            //     }
            while let Some(event) = stream.next().await {
                if ButtonEvent::Release(true) == *event {
                    rotate_background_colors(&a, &b, &c).await?;
                }
            }
            Ok(())
        }
    }));

    let hribbon: Arc<Ribbon> = RibbonParams::builder()
        .compositor(compositor.clone())
        .orientation(RibbonOrientation::Horizontal)
        .build()
        .add_panel(b()?, CellLimit::default())?
        .add_panel(b()?, CellLimit::default())?
        .add_panel(b()?, CellLimit::default())?
        .add_panel(b()?, CellLimit::default())?
        .add_panel(b()?, CellLimit::default())?
        .add_panel(b()?, CellLimit::default())?
        .add_panel(red_surface, CellLimit::default())?
        .add_panel(green_surface, CellLimit::default())?
        .add_panel(blue_surface, CellLimit::default())?
        .try_into()?;

    let vribbon: Arc<Ribbon> = RibbonParams::builder()
        .compositor(compositor.clone())
        .orientation(RibbonOrientation::Vertical)
        .build()
        .add_panel(hribbon, CellLimit::new(4., 100., None, None))?
        .add_panel(
            button,
            CellLimit::new(1., 50., Some(300.), Some(Vector2 { X: 0.5, Y: 0.8 })),
        )?
        .try_into()?;

    let layer_stack: Arc<LayerStack> = LayerStackParams::builder()
        .compositor(compositor.clone())
        .build()
        .push_panel(vribbon)
        .try_into()?;

    let root_visual = compositor.CreateContainerVisual()?;
    root_visual.SetSize(Vector2 { X: 800., Y: 600. })?;
    let channel = spawn_window_event_receiver(&pool, layer_stack, root_visual.clone())?;
    let window = Window::new(compositor, "demo", root_visual, channel);
    let _window = window.open()?;
    run_message_loop();

    // windows_app::bootstrap::uninitialize()?;

    Ok(())
}
