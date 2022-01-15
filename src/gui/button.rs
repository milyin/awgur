use super::{
    Background, BackgroundBuilder, LayerStack, Slot, SlotEvent, SlotEventData, SlotEventSource,
    SlotPlug,
};
use crate::async_handle_err;
use async_object::{Event, EventStream};
use async_object_derive::{async_object_decl, async_object_impl, async_object_with_events_decl};
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
    pub source: Event<SlotEvent>,
    pub data: ButtonEventData,
}

impl ButtonEvent {
    pub fn new(source: Event<SlotEvent>, data: ButtonEventData) -> Self {
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
    slot: Slot,
    _slot_plug: SlotPlug,
    pressed: bool,
}

impl ButtonImpl {
    fn new(compositor: &Compositor, slot: &mut Slot) -> crate::Result<Self> {
        let visual = compositor.CreateContainerVisual()?;
        let _slot_plug = slot.plug(visual.clone().into())?;
        let slot = Slot::new(visual, "button".into())?;
        Ok(Self {
            slot,
            _slot_plug,
            pressed: false,
        })
    }
}

#[async_object_impl(Button, WButton)]
impl ButtonImpl {
    pub fn slot(&self) -> Slot {
        self.slot.clone()
    }
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
    pub fn new(
        spawner: impl Spawn,
        compositor: &Compositor,
        slot: &mut Slot,
    ) -> crate::Result<Self> {
        let button = Self::create(ButtonImpl::new(compositor, slot)?)?;
        let future = {
            let mut stream = slot.create_slot_event_stream();
            let wbutton = button.downgrade();
            async move {
                while let Some(event) = stream.next().await {
                    if let Some(mut button) = wbutton.upgrade() {
                        button.translate_slot_event(event).await?
                    } else {
                        break;
                    }
                }
                Ok(())
            }
        };
        spawner.spawn(async_handle_err(future))?;
        Ok(button)
    }

    pub fn create_button_event_stream(&self) -> EventStream<ButtonEvent> {
        self.create_event_stream()
    }

    async fn translate_slot_event(&mut self, event: Event<SlotEvent>) -> crate::Result<()> {
        let data = match &event.as_ref().data {
            SlotEventData::MouseInput {
                in_slot,
                state,
                button,
            } => {
                if *button == MouseButton::Left {
                    if *state == ElementState::Pressed {
                        if *in_slot {
                            self.async_press().await;
                            self.send_event(ButtonEvent::new(
                                event.clone(),
                                ButtonEventData::Press,
                            ))
                            .await;
                        }
                    } else if *state == ElementState::Released {
                        if self.async_release().await {
                            self.send_event(ButtonEvent::new(
                                event.clone(),
                                ButtonEventData::Release(*in_slot),
                            ))
                            .await;
                        }
                    }
                }
                None
            }
            data => Some(data.clone()),
        };
        if let Some(data) = data {
            self.async_slot()
                .await
                .send_slot_event(SlotEvent::new(SlotEventSource::SlotEvent(event), data))
                .await;
        }
        Ok(())
    }
}

#[async_object_decl(pub ButtonDefaultDesign, pub WButtonDefaultDesign)]
struct ButtonDefaultDesignImpl {
    layer_stack: LayerStack,
    background: Background,
}

impl ButtonDefaultDesignImpl {
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

#[async_object_impl(ButtonDefaultDesign, WButtonDefaultDesign)]
impl ButtonDefaultDesignImpl {
    fn background(&self) -> Background {
        self.background.clone()
    }
}

impl ButtonDefaultDesign {
    pub fn new(
        spawner: impl Spawn + Clone,
        compositor: &Compositor,
        slot: &mut Slot,
        mut button_event_stream: EventStream<ButtonEvent>,
    ) -> crate::Result<Self> {
        let object =
            ButtonDefaultDesignImpl::new(spawner.clone(), compositor, slot, Colors::Magenta()?)?;
        let object = ButtonDefaultDesign::create(object);
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
