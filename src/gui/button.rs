use std::sync::Arc;

use super::{Background, BackgroundBuilder, EventSink, EventSource, LayerStack, Panel, PanelEvent};
use async_object::{CArc, EArc, EventBox, EventStream, WCArc};
use async_trait::async_trait;
use derive_weak::Weak;
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
    #[weak(WCArc)]
    core: CArc<Core>,
    events: EArc,
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

impl Button {
    pub fn new(compositor: &Compositor, skin: impl ButtonSkin + 'static) -> crate::Result<Self> {
        let container = compositor.CreateContainerVisual()?;
        let mut skin = skin;
        skin.attach(container.clone())?;
        let core = CArc::new(Core {
            skin: Box::new(skin),
            pressed: false,
        });
        Ok(Button {
            container,
            core,
            events: EArc::new(),
        })
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
        let mut skin = self.core.async_call(|v| v.skin_panel()).await;
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
                            self.core.async_call_mut(|v| v.press()).await;
                            self.events.send_event(ButtonEvent::Press, source).await;
                        }
                    } else if state == ElementState::Released {
                        let released = self.core.async_call_mut(|v| v.release()).await;
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
        self.core.id()
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
    events: EArc,
}

impl SimpleButtonSkin {
    pub fn new(compositor: Compositor, color: Color) -> crate::Result<Self> {
        let mut layer_stack = LayerStack::new(&compositor)?;
        let background = BackgroundBuilder::builder()
            .color(color)
            .round_corners(true)
            .build()
            .new(compositor)?;
        layer_stack.push_panel(background.clone())?;
        let earc = EArc::new();
        Ok(Self {
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
            ButtonEvent::Press => self.background.set_color(Colors::DarkMagenta()?)?,
            ButtonEvent::Release(_) => self.background.set_color(Colors::Magenta()?)?,
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
        EventStream::new(&self.events)
    }
}

impl Panel for SimpleButtonSkin {
    fn id(&self) -> usize {
        self.events.id()
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
