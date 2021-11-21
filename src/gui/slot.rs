use async_object::{EventStream, Keeper, Tag};
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
}

pub struct SlotPlug {
    slot_tag: SlotTag,
    plugged_visual: Visual,
}

impl SlotPlug {
    pub fn tag(&self) -> SlotTag {
        self.slot_tag.clone()
    }
}

impl Drop for SlotPlug {
    fn drop(&mut self) {
        if let Some(ref mut slot_container) = self.tag().slot_container() {
            let _ = slot_container
                .Children()
                .map(|c| c.Remove(&self.plugged_visual));
        }
    }
}

pub struct Slot(Keeper<SlotImpl>);

impl Slot {
    pub fn new(container: ContainerVisual, name: String) -> crate::Result<Self> {
        let keeper = Self(Keeper::new(SlotImpl::new(container, name)));
        Ok(keeper)
    }
    pub fn tag(&self) -> SlotTag {
        SlotTag(self.0.tag())
    }
    pub fn container(&self) -> ContainerVisual {
        self.0.read(|v| v.container.clone())
    }
    pub fn resize(&mut self, size: Vector2) -> crate::Result<()> {
        self.container().SetSize(size)?;
        self.0.send_event(SlotResized(size));
        self.0.send_event(WindowEvent::Resized(size.from_vector2()));
        Ok(())
    }
    pub fn send_window_event(&mut self, event: WindowEvent<'static>) -> crate::Result<()> {
        match event {
            WindowEvent::Resized(size) => self.resize(size.into_vector2())?,
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
}

#[derive(Clone)]
pub struct SlotResized(pub Vector2);

#[derive(Clone)]
pub struct SlotMouseInput;

#[derive(Clone)]
pub struct SlotCursorMoved(pub Vector2);

#[derive(Clone, PartialEq, Default)]
pub struct SlotTag(Tag<SlotImpl>);

impl SlotTag {
    pub async fn wait_for_destroy(&self) -> crate::Result<()> {
        let mut stream = EventStream::<()>::new(self.0.clone());
        while let Some(_) = stream.next().await {}
        Ok(())
    }
    pub fn on_window_event(&self) -> EventStream<WindowEvent<'static>> {
        EventStream::new(self.0.clone())
    }
    pub fn on_slot_resized(&self) -> EventStream<SlotResized> {
        EventStream::new(self.0.clone())
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
    pub fn name(&self) -> String {
        self.0
            .read(|v| v.name.clone())
            .unwrap_or("(dropped)/".into())
    }
    pub fn plug(&self, visual: Visual) -> crate::Result<SlotPlug> {
        if let Some(ref slot_container) = self.slot_container() {
            let size = slot_container.Size()?;
            visual.SetSize(size)?;
            slot_container.Children()?.InsertAtTop(visual.clone())?;
        }
        Ok(SlotPlug {
            slot_tag: self.clone(),
            plugged_visual: visual,
        })
    }
}

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
    source: SlotTag,
    destination: impl Send + Sync + 'static + TranslateWindowEvent,
) -> crate::Result<()> {
    let future = async move {
        while let Some(event) = source.on_window_event().next().await {
            if destination.translate_window_event(event).await?.is_none() {
                break;
            }
        }
        Ok(())
    };
    spawner.spawn(unwrap_err(future))?;
    Ok(())
}
