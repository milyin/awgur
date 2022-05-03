use super::{Background, BackgroundBuilder, LayerStack, Panel, PanelEvent, PanelEventData};
use crate::async_handle_err;
use async_object::{Event, EventStream};
use async_object_derive::{async_object_decl, async_object_impl, async_object_with_events_decl};
use async_trait::async_trait;
use futures::{
    task::{Spawn, SpawnExt},
    StreamExt,
};
use windows::UI::{
    Color, Colors,
    Composition::{Compositor, ContainerVisual},
};
use winit::event::{ElementState, MouseButton};

pub struct ButtonEvent {
    pub source: Event<PanelEvent>,
    pub data: ButtonEventData,
}

impl ButtonEvent {
    pub fn new(source: Event<PanelEvent>, data: ButtonEventData) -> Self {
        Self { source, data }
    }
}

#[derive(PartialEq)]
pub enum ButtonEventData {
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
    fn new(container: ContainerVisual) -> Self {
        let skin = DefaultButtonSkin::new()
        Self {
            container,
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
}

impl Button {
    pub fn new(compositor: &Compositor, skin: impl ButtonSkin + 'static) -> crate::Result<Self> {
        let container = compositor.CreateContainerVisual()?;
        let button = Self::create(ButtonImpl::new(container));
        Ok(button)
    }

    pub fn create_button_event_stream(&self) -> EventStream<ButtonEvent> {
        self.create_event_stream()
    }

    async fn translate_slot_event(&mut self, event: Event<PanelEvent>) -> crate::Result<()> {
        match event.as_ref().data {
            PanelEventData::MouseInput {
                in_slot,
                state,
                button,
            } => {
                if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        if in_slot {
                            self.async_press().await;
                            self.send_event(ButtonEvent::new(
                                event.clone(),
                                ButtonEventData::Press,
                            ))
                            .await;
                        }
                    } else if state == ElementState::Released {
                        if self.async_release().await {
                            self.send_event(ButtonEvent::new(
                                event.clone(),
                                ButtonEventData::Release(in_slot),
                            ))
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

#[async_trait]
pub trait ButtonSkin: Panel {
    async fn on_button_event(&mut self, event: PanelEvent) -> crate::Result<()>;
}

#[async_object_decl(pub DefaultButtonSkin, pub WDefaultButtonSkin)]
struct DefaultButtonSkinImpl {
    layer_stack: LayerStack,
    background: Background,
}

impl DefaultButtonSkinImpl {
    pub fn new(
        spawner: impl Spawn + Clone,
        compositor: &Compositor,
        slot: &mut Slot,
        color: Color,
    ) -> crate::Result<Self> {
        let mut layer_stack = LayerStack::new(spawner.clone(), compositor, slot)?;
        let mut slot = layer_stack.add_layer()?;
        let background = BackgroundBuilder::builder()
            .color(color)
            .round_corners(true)
            .build()
            .new(spawner, compositor, &mut slot)?;
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
}

impl DefaultButtonSkin {
    pub fn new(
        compositor: &Compositor,
        mut button_event_stream: EventStream<ButtonEvent>,
    ) -> crate::Result<Self> {
        let object =
            DefaultButtonSkinImpl::new(spawner.clone(), compositor, slot, Colors::Magenta()?)?;
        let object = DefaultButtonSkin::create(object);
        let future = {
            let wobject = object.downgrade();
            async move {
                while let Some(event) = button_event_stream.next().await {
                    if let Some(mut object) = wobject.upgrade() {
                        object.async_handle_button_event(event).await?
                    } else {
                        break;
                    }
                }
                Ok(())
            }
        };
        spawner.spawn(async_handle_err(future))?;
        Ok(object)
    }
    async fn async_handle_button_event(&mut self, event: Event<ButtonEvent>) -> crate::Result<()> {
        let mut background = self.async_background().await;
        match event.as_ref().data {
            ButtonEventData::Press => background.async_set_color(Colors::DarkMagenta()?).await?,
            ButtonEventData::Release(_) => background.async_set_color(Colors::Magenta()?).await?,
        }
        Ok(())
    }
}
