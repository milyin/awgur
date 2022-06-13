use super::{
    Background, BackgroundParams, EventSink, EventSource, LayerStack, LayerStackParams, Panel,
    PanelEvent,
};
use async_events::{EventBox, EventQueues, EventStream};
use async_std::sync::Arc;
use async_std::sync::RwLock;
use async_trait::async_trait;
use derive_weak::Weak;
use typed_builder::TypedBuilder;
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

#[derive(Clone, Weak)]
pub struct Button {
    container: ContainerVisual,
    core: Arc<RwLock<Core>>,
    events: Arc<EventQueues>,
}

#[derive(TypedBuilder)]
pub struct ButtonParams {
    compositor: Compositor,
    #[builder(setter(transform = |skin: impl ButtonSkin + 'static | Box::new(skin) as Box<dyn ButtonSkin>))]
    skin: Box<dyn ButtonSkin>,
}

impl ButtonParams {
    pub fn create(self) -> crate::Result<Button> {
        let container = self.compositor.CreateContainerVisual()?;
        let mut skin = self.skin;
        skin.attach(container.clone())?;
        let core = Arc::new(RwLock::new(Core {
            skin,
            pressed: false,
        }));
        Ok(Button {
            container,
            core,
            events: Arc::new(EventQueues::new()),
        })
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
    fn skin_panel(&self) -> Box<dyn Panel> {
        self.skin.clone_panel()
    }
}

impl EventSource<ButtonEvent> for Button {
    fn event_stream(&self) -> EventStream<ButtonEvent> {
        self.events.create_event_stream()
    }
}

impl EventSource<PanelEvent> for Button {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.events.create_event_stream()
    }
}

#[async_trait]
impl EventSink<PanelEvent> for Button {
    async fn on_event(
        &mut self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        let mut skin = self.core.read().await.skin_panel();
        skin.on_event(event.clone(), source.clone()).await?;
        self.events.send_event(event.clone(), source.clone()).await;

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
                            self.events.send_event(ButtonEvent::Press, source).await;
                        }
                    } else if state == ElementState::Released {
                        let released = self.core.write().await.release();
                        if released {
                            self.events
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
    fn id(&self) -> usize {
        Arc::as_ptr(&self.core) as usize
    }

    fn attach(&mut self, container: ContainerVisual) -> crate::Result<()> {
        container.Children()?.InsertAtTop(self.container.clone())?;
        Ok(())
    }
    fn detach(&mut self) -> crate::Result<()> {
        if let Ok(parent) = self.container.Parent() {
            parent.Children()?.Remove(&self.container)?;
        }
        Ok(())
    }

    fn clone_panel(&self) -> Box<dyn Panel> {
        Box::new(self.clone())
    }
}

pub trait ButtonSkin: Panel + EventSink<ButtonEvent> {}

#[derive(Clone)]
pub struct SimpleButtonSkin {
    layer_stack: LayerStack,
    background: Background,
    events: Arc<EventQueues>,
}

#[derive(TypedBuilder)]
pub struct SimpleButtonSkinParams {
    compositor: Compositor,
    color: Color,
}

impl SimpleButtonSkinParams {
    pub fn create(self) -> crate::Result<SimpleButtonSkin> {
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
        let earc = Arc::new(EventQueues::new());
        Ok(SimpleButtonSkin {
            layer_stack,
            background,
            events: earc,
        })
    }
}

#[async_trait]
impl EventSink<ButtonEvent> for SimpleButtonSkin {
    async fn on_event(
        &mut self,
        event: ButtonEvent,
        _: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
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
        &mut self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        self.layer_stack.on_event(event, source).await
    }
}

impl EventSource<PanelEvent> for SimpleButtonSkin {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.events.create_event_stream()
    }
}

impl Panel for SimpleButtonSkin {
    fn id(&self) -> usize {
        Arc::as_ptr(&self.events) as usize
    }

    fn attach(&mut self, container: ContainerVisual) -> crate::Result<()> {
        self.layer_stack.attach(container)
    }

    fn detach(&mut self) -> crate::Result<()> {
        self.layer_stack.detach()
    }

    fn clone_panel(&self) -> Box<dyn Panel> {
        Box::new(self.clone())
    }
}

impl ButtonSkin for SimpleButtonSkin {}
