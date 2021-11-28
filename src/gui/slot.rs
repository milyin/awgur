use async_object::{run, EventStream, Tag};
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

pub struct SlotImpl {
    container: ContainerVisual,
    name: String,
}

impl SlotImpl {
    pub fn new(container: ContainerVisual, name: String) -> Self {
        Self { container, name }
    }
    pub fn plug(&mut self, visual: &Visual) -> crate::Result<()> {
        let size = self.container.Size()?;
        visual.SetSize(size)?;
        self.container.Children()?.InsertAtTop(visual.clone())?;
        Ok(())
    }
}

pub struct SlotPlug {
    slot: Slot,
    plugged_visual: Visual,
}

impl SlotPlug {
    pub fn slot(&self) -> Slot {
        self.slot.clone()
    }
}

impl Drop for SlotPlug {
    fn drop(&mut self) {
        if let Some(ref mut slot_container) = self.slot().slot_container() {
            let _ = slot_container
                .Children()
                .map(|c| c.Remove(&self.plugged_visual));
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Slot(Tag<SlotImpl>);

impl Slot {
    pub fn new(pool: impl Spawn, container: ContainerVisual, name: String) -> crate::Result<Self> {
        let slot = Self(run(pool, SlotImpl::new(container, name))?);
        Ok(slot)
    }
    pub fn container_sync(&self) -> Option<ContainerVisual> {
        self.0.read(|v| v.container.clone())
    }
    pub fn resize_sync(&mut self, size: Vector2) -> crate::Result<()> {
        println!(
            "{} {},{}",
            self.0.read(|v| v.name.clone()).unwrap(),
            size.X,
            size.Y
        );
        self.container_sync().map(|v| v.SetSize(size)).transpose()?;
        self.0.send_event(SlotResized(size));
        self.0.send_event(WindowEvent::Resized(size.from_vector2()));
        Ok(())
    }
    pub fn send_window_event_sync(&mut self, event: WindowEvent<'static>) -> crate::Result<()> {
        match event {
            WindowEvent::Resized(size) => self.resize_sync(size.into_vector2())?,
            WindowEvent::CursorMoved { position, .. } => {
                self.0.send_event(SlotCursorMoved(position.into_vector2()));
                self.0.send_event(event);
            }
            event @ WindowEvent::MouseInput { .. } => {
                self.0.send_event(SlotMouseInput);
                self.0.send_event(event);
            }
            event => self.0.send_event(event),
        }
        Ok(())
    }
    pub async fn wait_for_destroy(&self) -> crate::Result<()> {
        let mut stream = EventStream::<()>::new(self.0.clone());
        while let Some(_) = stream.next().await {}
        Ok(())
    }
    pub fn on_window_event(&self) -> EventStream<WindowEvent<'static>> {
        EventStream::new(self.0.clone())
    }
    pub fn on_slot_resized(&self) -> EventStream<SlotResized> {
        let subscription = EventStream::new(self.0.clone());
        let _ = self.resend_slot_resized();
        subscription
    }
    pub fn on_slot_mouse_input(&self) -> EventStream<SlotMouseInput> {
        EventStream::new(self.0.clone())
    }
    pub fn on_slot_cursor_moved(&self) -> EventStream<SlotCursorMoved> {
        EventStream::new(self.0.clone())
    }
    pub fn slot_container(&self) -> Option<ContainerVisual> {
        self.0.read(|v| v.container.clone())
    }
    pub fn resend_slot_resized(&self) -> crate::Result<Option<()>> {
        if let Some(container) = self.slot_container() {
            dbg!(container.Size()?);
            self.0.send_event(SlotResized(container.Size()?));
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
    pub fn name(&self) -> String {
        self.0
            .read(|v| v.name.clone())
            .unwrap_or("(dropped)/".into())
    }
    pub fn plug(&self, visual: Visual) -> crate::Result<SlotPlug> {
        self.0.write(|v| v.plug(&visual));
        Ok(SlotPlug {
            slot: self.clone(),
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
    async fn translate_window_event(
        &self,
        event: WindowEvent<'static>,
    ) -> crate::Result<Option<()>>;
    async fn name(&self) -> String;
}

pub fn spawn_translate_window_events(
    spawner: impl Spawn,
    source: Slot,
    destination: impl Send + Sync + 'static + TranslateWindowEvent,
) -> crate::Result<()> {
    let future = async move {
        while let Some(event) = source.on_window_event().next().await {
            if destination
                .translate_window_event(event.as_ref().clone())
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
