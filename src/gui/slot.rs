use async_object::{EventStream, Keeper, Tag};
use futures::{
    task::{Spawn, SpawnExt},
    StreamExt,
};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use windows::{
    Foundation::Numerics::Vector2,
    UI::Composition::{ContainerVisual, Visual},
};
use winit::event::WindowEvent;

use crate::unwrap_err;

#[derive(Clone)]
pub struct Slot {
    tag: SlotTag,
    container: ContainerVisual,
}

impl Slot {
    fn new(container: ContainerVisual) -> crate::Result<Self> {
        Ok(Self {
            tag: SlotTag::default(),
            container,
        })
    }
    pub fn plug(&mut self, visual: Visual) -> crate::Result<SlotPlug> {
        let size = self.container.Size()?;
        visual.SetSize(size)?;
        self.container.Children()?.InsertAtTop(visual.clone())?;
        Ok(SlotPlug {
            tag: self.tag.clone(),
            container: self.container.clone(),
            visual,
        })
    }
}

pub struct SlotPlug {
    tag: SlotTag,
    container: ContainerVisual,
    visual: Visual,
}

impl SlotPlug {
    pub fn tag(&self) -> SlotTag {
        self.tag.clone()
    }
}

impl From<SlotPlug> for SlotTag {
    fn from(plug: SlotPlug) -> Self {
        plug.tag()
    }
}

impl Drop for SlotPlug {
    fn drop(&mut self) {
        let _ = self.container.Children().map(|c| c.Remove(&self.visual));
    }
}

pub struct SlotKeeper(Keeper<Slot, ContainerVisual>);

impl SlotKeeper {
    pub fn new(container: ContainerVisual) -> crate::Result<Self> {
        let slot = Slot::new(container.clone())?;
        let keeper = Self(Keeper::new_with_shared(
            slot,
            Arc::new(RwLock::new(container)),
        ));
        keeper.get_mut().tag = keeper.tag();
        Ok(keeper)
    }
    pub fn tag(&self) -> SlotTag {
        SlotTag(self.0.tag())
    }
    pub fn get(&self) -> RwLockReadGuard<'_, Slot> {
        self.0.get()
    }
    pub fn get_mut(&self) -> RwLockWriteGuard<'_, Slot> {
        self.0.get_mut()
    }
    pub fn container(&self) -> crate::Result<ContainerVisual> {
        Ok(self.0.clone_shared())
    }
    pub fn resize(&mut self, size: Vector2) -> crate::Result<()> {
        self.container()?.SetSize(size)?;
        self.0.send_event(SlotResize(size));
        Ok(())
    }
    pub fn translate_window_event(&mut self, event: WindowEvent<'static>) -> crate::Result<()> {
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
pub struct SlotTag(Tag<Slot, ContainerVisual>);

impl SlotTag {
    pub async fn wait_for_destroy(&self) -> crate::Result<()> {
        let mut stream = EventStream::<()>::new(self.0.clone());
        while let Some(_) = stream.next().await {}
        Ok(())
    }
    pub fn plug(&self, visual: Visual) -> crate::Result<SlotPlug> {
        Ok(self.0.call_mut(|v| v.plug(visual))??)
    }
    pub fn on_window_event(&self) -> EventStream<WindowEvent<'static>> {
        EventStream::new(self.0.clone())
    }
    pub fn on_slot_resize(&self) -> EventStream<SlotResize> {
        EventStream::new(self.0.clone())
    }
}

pub trait TranslateWindowEvent {
    fn translate_window_event(&self, event: WindowEvent) -> crate::Result<()>;
}

pub fn spawn_translate_window_events(
    spawner: impl Spawn,
    source: SlotTag,
    destination: impl Send + 'static + TranslateWindowEvent,
) -> crate::Result<()> {
    let future = async move {
        while let Some(event) = source.on_window_event().next().await {
            match destination.translate_window_event(event) {
                Err(crate::Error::AsyncObject(async_object::Error::Destroyed)) => return Ok(()),
                e @ Err(_) => return e,
                _ => (),
            }
        }
        Ok(())
    };
    spawner.spawn(unwrap_err(future))?;
    Ok(())
}
