use async_object::{EventStream, Keeper, Tag};
use async_trait::async_trait;
use futures::{
    task::{Spawn, SpawnExt},
    StreamExt,
};
use std::sync::{Arc, RwLock};
use windows::{
    Foundation::Numerics::Vector2,
    UI::Composition::{ContainerVisual, Visual},
};
use winit::event::WindowEvent;

use crate::unwrap_err;

pub struct SlotPlug {
    slot_tag: SlotTag,
    slot_container: Option<ContainerVisual>,
    plugged_visual: Visual,
}

impl SlotPlug {
    pub fn tag(&self) -> SlotTag {
        self.slot_tag.clone()
    }
}

impl From<SlotPlug> for SlotTag {
    fn from(plug: SlotPlug) -> Self {
        plug.tag()
    }
}

impl Drop for SlotPlug {
    fn drop(&mut self) {
        if let Some(ref mut slot_container) = self.slot_container {
            let _ = slot_container
                .Children()
                .map(|c| c.Remove(&self.plugged_visual));
        }
    }
}

pub struct Slot(Keeper<(), ContainerVisual>);

impl Slot {
    pub fn new(container: ContainerVisual) -> crate::Result<Self> {
        let keeper = Self(Keeper::new_with_shared(
            (),
            Arc::new(RwLock::new(container)),
        ));
        Ok(keeper)
    }
    pub fn tag(&self) -> SlotTag {
        SlotTag(self.0.tag())
    }
    pub fn container(&self) -> crate::Result<ContainerVisual> {
        Ok(self.0.clone_shared())
    }
    pub fn resize(&mut self, size: Vector2) -> crate::Result<()> {
        self.container()?.SetSize(size)?;
        self.0.send_event(SlotResize(size));
        Ok(())
    }
    pub fn send_window_event(&mut self, event: WindowEvent<'static>) -> crate::Result<()> {
        match &event {
            WindowEvent::Resized(size) => {
                let size = Vector2 {
                    X: size.width as f32,
                    Y: size.height as f32,
                };
                self.resize(size)?;
            }
            _ => (),
        }
        self.0.send_event(event);
        Ok(())
    }
}

#[derive(Clone)]
pub struct SlotResize(pub Vector2);

#[derive(Clone, PartialEq, Default)]
pub struct SlotTag(Tag<(), ContainerVisual>);

impl SlotTag {
    pub async fn wait_for_destroy(&self) -> crate::Result<()> {
        let mut stream = EventStream::<()>::new(self.0.clone());
        while let Some(_) = stream.next().await {}
        Ok(())
    }
    pub fn on_window_event(&self) -> EventStream<WindowEvent<'static>> {
        EventStream::new(self.0.clone())
    }
    pub fn on_slot_resize(&self) -> EventStream<SlotResize> {
        EventStream::new(self.0.clone())
    }
    pub fn plug(&self, visual: Visual) -> crate::Result<SlotPlug> {
        let slot_container = self.0.clone_shared();
        if let Some(ref slot_container) = slot_container {
            let size = slot_container.Size()?;
            visual.SetSize(size)?;
            slot_container.Children()?.InsertAtTop(visual.clone())?;
        }
        Ok(SlotPlug {
            slot_tag: self.clone(),
            slot_container,
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
