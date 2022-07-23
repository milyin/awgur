use super::{attach, ArcPanel};
use super::{
    Background, BackgroundParams, EventSink, EventSource, LayerStack, LayerStackParams, Panel,
    PanelEvent,
};
use async_event_streams::{EventBox, EventStream, EventStreams};
use async_std::sync::Arc;
use async_std::sync::RwLock;
use async_trait::async_trait;
use typed_builder::TypedBuilder;
use windows::UI::Composition::Visual;
use windows::UI::{
    Color, Colors,
    Composition::{Compositor, ContainerVisual},
};
use winit::event::{ElementState, MouseButton};

#[derive(PartialEq)]
pub enum ButtonEvent {
    Press,
    Release(bool),
}

struct Core {
    skin: Box<dyn ButtonSkin>,
    pressed: bool,
}

pub struct Button {
    container: ContainerVisual,
    core: RwLock<Core>,
    panel_events: EventStreams<PanelEvent>,
    button_events: EventStreams<ButtonEvent>,
}

#[derive(TypedBuilder)]
pub struct ButtonParams {
    compositor: Compositor,
    #[builder(setter(transform = |skin: impl ButtonSkin + 'static | Box::new(skin) as Box<dyn ButtonSkin>))]
    skin: Box<dyn ButtonSkin>,
}

impl ButtonParams {
    pub fn create(self) -> crate::Result<Arc<Button>> {
        let container = self.compositor.CreateContainerVisual()?;
        let skin = self.skin;
        attach(&container, &skin)?;
        let core = RwLock::new(Core {
            skin,
            pressed: false,
        });
        Ok(Arc::new(Button {
            container,
            core,
            panel_events: EventStreams::new(),
            button_events: EventStreams::new(),
        }))
    }
}

impl Core {
    fn press(&mut self) {
        self.pressed = true;
    }
    fn release(&mut self) -> bool {
        let pressed = self.pressed;
        self.pressed = false;
        pressed
    }
    fn skin_panel(&self) -> Box<dyn ArcPanel> {
        self.skin.clone_box()
    }
}

impl EventSource<ButtonEvent> for Button {
    fn event_stream(&self) -> EventStream<ButtonEvent> {
        self.button_events.create_event_stream()
    }
}

impl EventSource<PanelEvent> for Button {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.panel_events.create_event_stream()
    }
}

#[async_trait]
impl EventSink<PanelEvent> for Button {
    async fn on_event(
        &self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        let skin = self.core.read().await.skin_panel();
        skin.on_event(event.clone(), source.clone()).await?;
        self.panel_events
            .send_event(event.clone(), source.clone())
            .await;

        match event {
            PanelEvent::MouseInput {
                in_slot,
                state,
                button,
            } => {
                if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        if in_slot {
                            self.core.write().await.press();
                            self.button_events
                                .send_event(ButtonEvent::Press, source)
                                .await;
                        }
                    } else if state == ElementState::Released {
                        let released = self.core.write().await.release();
                        if released {
                            self.button_events
                                .send_event(ButtonEvent::Release(in_slot), source)
                                .await;
                        }
                    }
                }
            }
            _ => {}
        };
        Ok(())
    }
}

impl Panel for Button {
    fn outer_frame(&self) -> Visual {
        self.container.clone().into()
    }
}

pub trait ButtonSkin: ArcPanel + EventSink<ButtonEvent> {}

pub struct SimpleButtonSkin {
    layer_stack: LayerStack,
    background: Arc<Background>,
    panel_events: EventStreams<PanelEvent>,
}

#[derive(TypedBuilder)]
pub struct SimpleButtonSkinParams {
    compositor: Compositor,
    color: Color,
}

impl SimpleButtonSkinParams {
    pub fn create(self) -> crate::Result<Arc<SimpleButtonSkin>> {
        let background = BackgroundParams::builder()
            .color(self.color)
            .round_corners(true)
            .compositor(self.compositor.clone())
            .build()
            .create()?;
        let layer_stack = LayerStackParams::builder()
            .compositor(self.compositor)
            .build()
            .push_panel(background.clone())
            .create()?;
        Ok(Arc::new(SimpleButtonSkin {
            layer_stack,
            background,
            panel_events: EventStreams::new(),
        }))
    }
}

#[async_trait]
impl EventSink<ButtonEvent> for SimpleButtonSkin {
    async fn on_event(&self, event: ButtonEvent, _: Option<Arc<EventBox>>) -> crate::Result<()> {
        match event {
            ButtonEvent::Press => self.background.set_color(Colors::DarkMagenta()?).await?,
            ButtonEvent::Release(_) => self.background.set_color(Colors::Magenta()?).await?,
        }
        Ok(())
    }
}

#[async_trait]
impl EventSink<PanelEvent> for SimpleButtonSkin {
    async fn on_event(
        &self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        self.layer_stack.on_event(event, source).await
    }
}

impl EventSource<PanelEvent> for SimpleButtonSkin {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.panel_events.create_event_stream()
    }
}

impl Panel for SimpleButtonSkin {
    fn outer_frame(&self) -> Visual {
        self.layer_stack.outer_frame()
    }
}

impl ButtonSkin for Arc<SimpleButtonSkin> {}
