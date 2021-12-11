use async_object::EventStream;
use async_object_derive::{async_object_impl, async_object_with_events_decl};
use async_trait::async_trait;
use futures::{
    task::{Spawn, SpawnExt},
    StreamExt,
};
use windows::{
    Foundation::Numerics::Vector2,
    UI::Composition::{ContainerVisual, Visual},
};
use winit::event::WindowEvent;

use crate::unwrap_err;

use super::{FromVector2, IntoVector2};

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
    pub fn plug_internal(&mut self, visual: &Visual) -> crate::Result<()> {
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
    pub fn new(pool: impl Spawn, container: ContainerVisual, name: String) -> crate::Result<Self> {
        let slot = Self::create(SlotImpl::new(container, name), pool)?;
        Ok(slot)
    }
    pub fn resize(&mut self, size: Vector2) -> crate::Result<()> {
        println!("{} {},{}", self.name(), size.X, size.Y);
        self.container().SetSize(size)?;
        self.send_event(SlotResized(size));
        self.send_event(WindowEvent::Resized(size.from_vector2()));
        Ok(())
    }
    pub fn send_window_event(&mut self, event: WindowEvent<'static>) -> crate::Result<()> {
        match event {
            WindowEvent::Resized(size) => self.resize(size.into_vector2())?,
            WindowEvent::CursorMoved { position, .. } => {
                self.send_event(SlotCursorMoved(position.into_vector2()));
                self.send_event(event);
            }
            event @ WindowEvent::MouseInput { .. } => {
                self.send_event(SlotMouseInput);
                self.send_event(event);
            }
            event => self.send_event(event),
        }
        Ok(())
    }
    pub async fn async_wait_for_destroy(&self) -> crate::Result<()> {
        let mut stream = self.create_event_stream::<()>();
        while let Some(_) = stream.next().await {}
        Ok(())
    }
    pub fn on_window_event(&self) -> EventStream<WindowEvent<'static>> {
        self.create_event_stream()
    }
    pub fn on_slot_resized(&self) -> EventStream<SlotResized> {
        let subscription = self.create_event_stream();
        let _ = self.resend_slot_resized();
        subscription
    }
    pub fn on_slot_mouse_input(&self) -> EventStream<SlotMouseInput> {
        self.create_event_stream()
    }
    pub fn on_slot_cursor_moved(&self) -> EventStream<SlotCursorMoved> {
        self.create_event_stream()
    }
    pub fn resend_slot_resized(&self) -> crate::Result<Option<()>> {
        let container = self.container();
        dbg!(container.Size()?);
        self.send_event(SlotResized(container.Size()?));
        Ok(Some(()))
    }
    pub fn plug(&mut self, visual: Visual) -> crate::Result<SlotPlug> {
        self.plug_internal(&visual)?;
        Ok(SlotPlug {
            slot: self.downgrade(),
            plugged_visual: visual,
        })
    }
}

#[derive(Clone)]
pub struct SlotResized(pub Vector2);

#[derive(Clone)]
pub struct SlotMouseInput;

#[derive(Clone)]
pub struct SlotCursorMoved(pub Vector2);

#[async_trait]
pub trait TranslateWindowEvent {
    async fn async_translate_window_event(
        &mut self,
        event: WindowEvent<'static>,
    ) -> crate::Result<Option<()>>;
}

pub fn spawn_translate_window_events(
    spawner: impl Spawn,
    source: Slot,
    mut destination: impl Send + Sync + 'static + TranslateWindowEvent,
) -> crate::Result<()> {
    let future = async move {
        while let Some(event) = source.on_window_event().next().await {
            if destination
                .async_translate_window_event(event.as_ref().clone())
                .await?
                .is_none()
            {
                break;
            }
        }
        Ok(())
    };
    spawner.spawn(unwrap_err(future))?;
    Ok(())
}
