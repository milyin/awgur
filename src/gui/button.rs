use std::sync::Arc;

use super::{Background, BackgroundBuilder, EventSink, EventSource, LayerStack, Panel, PanelEvent};
use async_object::{EventBox, EventStream};
use async_object_derive::{async_object_impl, async_object_with_events_decl};
use async_trait::async_trait;
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

#[async_object_with_events_decl(pub Button, pub WButton)]
struct ButtonImpl {
    container: ContainerVisual,
    skin: Box<dyn ButtonSkin>,
    pressed: bool,
}

impl ButtonImpl {
    fn new(container: ContainerVisual, skin: Box<dyn ButtonSkin>) -> Self {
        Self {
            container,
            skin,
            pressed: false,
        }
    }
}

#[async_object_impl(Button, WButton)]
impl ButtonImpl {
    pub fn press(&mut self) {
        self.pressed = true;
    }
    pub fn release(&mut self) -> bool {
        let pressed = self.pressed;
        self.pressed = false;
        pressed
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
    fn skin(&self) -> Box<dyn Panel> {
        self.skin.clone_panel()
    }
}

impl Button {
    pub fn new(compositor: &Compositor, skin: impl ButtonSkin + 'static) -> crate::Result<Self> {
        let container = compositor.CreateContainerVisual()?;
        let mut skin = skin;
        skin.attach(container.clone())?;
        let button = Self::create(ButtonImpl::new(container, Box::new(skin)));
        Ok(button)
    }

    async fn on_panel_event(
        &mut self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        match event {
            PanelEvent::MouseInput {
                in_slot,
                state,
                button,
            } => {
                if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        if in_slot {
                            self.async_press().await;
                            self.send_event(ButtonEvent::Press, source).await;
                        }
                    } else if state == ElementState::Released {
                        if self.async_release().await {
                            self.send_event(ButtonEvent::Release(in_slot), source).await;
                        }
                    }
                }
            }
            _ => {}
        };
        Ok(())
    }
}

impl EventSource<ButtonEvent> for Button {
    fn event_stream(&self) -> EventStream<ButtonEvent> {
        self.create_event_stream()
    }
}

impl EventSource<PanelEvent> for Button {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.create_event_stream()
    }
}

#[async_trait]
impl EventSink<PanelEvent> for Button {
    async fn on_event(
        &mut self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        self.async_skin()
            .await
            .on_event(event.clone(), source.clone())
            .await?;
        self.on_panel_event(event.clone(), source.clone()).await?;
        self.send_event(event, source).await;
        Ok(())
    }
}

impl Panel for Button {
    fn id(&self) -> usize {
        self.id()
    }

    fn attach(&mut self, container: ContainerVisual) -> crate::Result<()> {
        self.attach(container)
    }

    fn detach(&mut self) -> crate::Result<()> {
        self.detach()
    }

    fn clone_panel(&self) -> Box<dyn Panel> {
        Box::new(self.clone())
    }
}

pub trait ButtonSkin: Panel + EventSink<ButtonEvent> {}

#[async_object_with_events_decl(pub DefaultButtonSkin, pub WDefaultButtonSkin)]
struct DefaultButtonSkinImpl {
    layer_stack: LayerStack,
    background: Background,
}

impl DefaultButtonSkinImpl {
    pub fn new(compositor: Compositor, color: Color) -> crate::Result<Self> {
        let mut layer_stack = LayerStack::new(compositor.clone())?;
        let background = BackgroundBuilder::builder()
            .color(color)
            .round_corners(true)
            .build()
            .new(compositor)?;
        layer_stack.push_panel(background.clone())?;
        Ok(Self {
            layer_stack,
            background,
        })
    }
}

#[async_object_impl(DefaultButtonSkin, WDefaultButtonSkin)]
impl DefaultButtonSkinImpl {
    fn background(&self) -> Background {
        self.background.clone()
    }
    fn layer_stack(&self) -> LayerStack {
        self.layer_stack.clone()
    }
}

impl DefaultButtonSkin {
    pub fn new(compositor: Compositor) -> crate::Result<Self> {
        let object = DefaultButtonSkinImpl::new(compositor, Colors::Magenta()?)?;
        let object = DefaultButtonSkin::create(object);
        Ok(object)
    }
    async fn on_button_event(&mut self, event: ButtonEvent) -> crate::Result<()> {
        match event {
            ButtonEvent::Press => {
                self.async_background()
                    .await
                    .async_set_color(Colors::DarkMagenta()?)
                    .await?
            }
            ButtonEvent::Release(_) => {
                self.async_background()
                    .await
                    .async_set_color(Colors::Magenta()?)
                    .await?
            }
        }
        Ok(())
    }
}

#[async_trait]
impl EventSink<ButtonEvent> for DefaultButtonSkin {
    async fn on_event(
        &mut self,
        event: ButtonEvent,
        _: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        self.on_button_event(event).await
    }
}

#[async_trait]
impl EventSink<PanelEvent> for DefaultButtonSkin {
    async fn on_event(
        &mut self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        self.async_layer_stack().await.on_event(event, source).await
    }
}

impl EventSource<PanelEvent> for DefaultButtonSkin {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.create_event_stream()
    }
}

impl Panel for DefaultButtonSkin {
    fn id(&self) -> usize {
        self.id()
    }

    fn attach(&mut self, container: ContainerVisual) -> crate::Result<()> {
        self.layer_stack().attach(container)
    }

    fn detach(&mut self) -> crate::Result<()> {
        self.layer_stack().detach()
    }

    fn clone_panel(&self) -> Box<dyn Panel> {
        Box::new(self.clone())
    }
}

impl ButtonSkin for DefaultButtonSkin {}
