use async_object::{Event, EventStream};
use async_object_derive::{async_object_impl, async_object_with_events_decl};
use async_trait::async_trait;
use futures::StreamExt;
use windows::{
    Foundation::Numerics::Vector2,
    UI::Composition::{ContainerVisual, Visual},
};
use winit::event::{ElementState, MouseButton, WindowEvent};

use super::IntoVector2;

#[derive(Clone)]
pub enum SlotEventData {
    Resized(Vector2),
    CursorMoved(Vector2),
    MouseInput {
        in_slot: bool,
        state: ElementState,
        button: MouseButton,
    },
    Empty,
}

#[derive(Clone)]
pub enum SlotEventSource {
    WindowEvent(WindowEvent<'static>),
    SlotEvent(Event<SlotEvent>),
    None,
}

#[derive(Clone)]
pub struct SlotEvent {
    pub source: SlotEventSource,
    pub data: SlotEventData,
}

impl SlotEvent {
    pub fn from_window_event(event: WindowEvent<'static>) -> Self {
        let data = match &event {
            WindowEvent::Resized(size) => SlotEventData::Resized(size.into_vector2()),
            WindowEvent::CursorMoved { position, .. } => {
                SlotEventData::CursorMoved(position.into_vector2())
            }
            WindowEvent::MouseInput { state, button, .. } => SlotEventData::MouseInput {
                in_slot: true,
                state: *state,
                button: *button,
            },
            _ => SlotEventData::Empty,
        };
        Self {
            source: SlotEventSource::WindowEvent(event),
            data,
        }
    }

    pub fn new(source: SlotEventSource, data: SlotEventData) -> Self {
        Self { source, data }
    }
}

#[async_object_with_events_decl(pub Slot, pub WSlot)]
pub struct SlotImpl {
    container: ContainerVisual,
    name: String,
}

impl SlotImpl {
    pub fn new(container: ContainerVisual, name: String) -> Self {
        Self { container, name }
    }
}

#[async_object_impl(Slot, WSlot)]
impl SlotImpl {
    fn plug_internal(&mut self, visual: &Visual) -> crate::Result<()> {
        let size = self.container.Size()?;
        visual.SetSize(size)?;
        self.container.Children()?.InsertAtTop(visual.clone())?;
        Ok(())
    }
    pub fn container(&self) -> ContainerVisual {
        self.container.clone()
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

pub struct SlotPlug {
    slot: WSlot,
    plugged_visual: Visual,
}

impl SlotPlug {
    pub fn new(plugged_visual: Visual) -> Self {
        Self {
            slot: WSlot::default(),
            plugged_visual,
        }
    }

    pub fn slot(&self) -> WSlot {
        self.slot.clone()
    }
}

impl Drop for SlotPlug {
    fn drop(&mut self) {
        if let Some(slot_container) = self.slot().container() {
            let _ = slot_container
                .Children()
                .map(|c| c.Remove(&self.plugged_visual));
        }
    }
}

impl Slot {
    pub fn new(container: ContainerVisual, name: String) -> crate::Result<Self> {
        let slot = Self::create(SlotImpl::new(container, name));
        Ok(slot)
    }
    pub async fn send_slot_event(&self, event: SlotEvent) {
        self.send_event(event).await
    }
    pub async fn async_wait_for_destroy(&self) -> crate::Result<()> {
        let mut stream = self.create_event_stream::<()>();
        while let Some(_) = stream.next().await {}
        Ok(())
    }
    pub fn create_slot_event_stream(&self) -> EventStream<SlotEvent> {
        self.create_event_stream()
    }
    pub fn plug(&mut self, visual: Visual) -> crate::Result<SlotPlug> {
        self.plug_internal(&visual)?;
        Ok(SlotPlug {
            slot: self.downgrade(),
            plugged_visual: visual,
        })
    }
}
#[async_trait]
pub trait Plug: Send + Sync {
    fn get_visual(&self) -> Visual;
    async fn on_slot_event(&mut self, event: SlotEvent) -> crate::Result<()>;
    fn clone_box(&self) -> Box<dyn Plug>;
}

impl Clone for Box<dyn Plug> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl<T: Plug> PartialEq<T> for Box<dyn Plug> {
    fn eq(&self, other: &T) -> bool {
        self.get_visual() == other.get_visual()
    }
}
