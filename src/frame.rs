use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use async_object::{Keeper, Tag};
use futures::{executor::ThreadPool, Future};
use windows::UI::Composition::{Compositor, ContainerVisual};

use crate::{
    slot::{SlotKeeper, SlotTag},
    slot_event::{MouseLeftPressed, MouseLeftPressedFocused, SendSlotEvent},
    SlotSize,
};

pub struct FrameShared {
    thread_pool: ThreadPool,
    compositor: Compositor,
    frame_visual: ContainerVisual,
}
pub struct Frame {
    shared: Arc<RwLock<FrameShared>>,
    slots: Vec<SlotKeeper>,
}

impl Frame {
    fn new(thread_pool: ThreadPool) -> crate::Result<Self> {
        let compositor = Compositor::new()?;
        let frame_visual = compositor.CreateContainerVisual()?;
        let shared = Arc::new(RwLock::new(FrameShared {
            thread_pool,
            compositor,
            frame_visual,
        }));
        Ok(Self {
            shared,
            slots: Vec::new(),
        })
    }
    fn shared(&self) -> Arc<RwLock<FrameShared>> {
        self.shared.clone()
    }
    fn compositor(&self) -> Compositor {
        self.shared.read().unwrap().compositor.clone()
    }
    fn frame_visual(&self) -> ContainerVisual {
        self.shared.read().unwrap().frame_visual.clone()
    }

    fn open_slot(&mut self) -> crate::Result<SlotTag> {
        let compositor = self.compositor();
        let container = compositor.CreateContainerVisual()?;
        container.SetSize(self.frame_visual().Size()?)?;
        self.shared
            .read()
            .unwrap()
            .frame_visual
            .Children()?
            .InsertAtTop(container.clone())?;
        let slot_keeper = SlotKeeper::new(container)?;
        let slot = slot_keeper.tag();
        self.slots.push(slot_keeper);
        Ok(slot)
    }

    pub fn close_slot(&mut self, slot: SlotTag) -> crate::Result<()> {
        if let Some(index) = self.slots.iter().position(|v| v.tag() == slot) {
            let slot = self.slots.remove(index);
            self.shared
                .read()
                .unwrap()
                .frame_visual
                .Children()?
                .Remove(slot.container()?)?;
        }
        Ok(())
    }
}

impl SendSlotEvent for Frame {
    fn send_size(&mut self, size: SlotSize) -> crate::Result<()> {
        self.frame_visual().SetSize(size.0)?;
        for slot in &mut self.slots {
            slot.send_size(size.clone())?;
        }
        Ok(())
    }

    fn send_mouse_left_pressed(&mut self, event: MouseLeftPressed) -> crate::Result<()> {
        for slot in &mut self.slots {
            slot.send_mouse_left_pressed(event.clone())?;
        }
        Ok(())
    }

    fn send_mouse_left_pressed_focused(
        &mut self,
        event: MouseLeftPressedFocused,
    ) -> crate::Result<()> {
        if let Some(slot) = self.slots.last_mut() {
            slot.send_mouse_left_pressed_focused(event)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct FrameKeeper(Keeper<Frame, FrameShared>);

impl FrameKeeper {
    pub fn new(thread_pool: ThreadPool) -> crate::Result<Self> {
        let frame = Frame::new(thread_pool)?;
        let shared = frame.shared();
        let keeper = Keeper::new_with_shared(frame, shared);
        Ok(Self(keeper))
    }
    pub fn tag(&self) -> FrameTag {
        FrameTag(self.0.tag())
    }
    pub fn get(&self) -> RwLockReadGuard<'_, Frame> {
        self.0.get()
    }
    pub fn get_mut(&self) -> RwLockWriteGuard<'_, Frame> {
        self.0.get_mut()
    }
}

#[derive(Clone, PartialEq)]
pub struct FrameTag(Tag<Frame, FrameShared>);

impl FrameTag {
    pub fn compositor(&self) -> crate::Result<Compositor> {
        Ok(self.0.read_shared(|v| v.compositor.clone())?)
    }
    pub fn frame_visual(&self) -> crate::Result<ContainerVisual> {
        Ok(self.0.read_shared(|v| v.frame_visual.clone())?)
    }
    pub fn thread_pool(&self) -> crate::Result<ThreadPool> {
        Ok(self.0.read_shared(|v| v.thread_pool.clone())?)
    }
    pub fn thread_spawn<Fut>(&self, future: Fut) -> crate::Result<()>
    where
        Fut: Future<Output = crate::Result<()>> + Send + 'static,
    {
        self.thread_pool()?.spawn_ok(async {
            future.await.unwrap() // TODO: store error somethere (thread_local? special inrerface in tag?)
        });
        Ok(())
    }
    pub fn open_slot(&self) -> crate::Result<SlotTag> {
        self.0.call_mut(|frame| frame.open_slot())?
    }
    pub fn close_slot(&self, slot: SlotTag) -> crate::Result<()> {
        self.0.call_mut(|frame| frame.close_slot(slot))?
    }
}

impl SendSlotEvent for FrameTag {
    fn send_size(&mut self, size: SlotSize) -> crate::Result<()> {
        self.0.call_mut(|frame| frame.send_size(size))?
    }

    fn send_mouse_left_pressed(&mut self, event: MouseLeftPressed) -> crate::Result<()> {
        self.0
            .call_mut(|frame| frame.send_mouse_left_pressed(event))?
    }

    fn send_mouse_left_pressed_focused(
        &mut self,
        event: MouseLeftPressedFocused,
    ) -> crate::Result<()> {
        self.0
            .call_mut(|frame| frame.send_mouse_left_pressed_focused(event))?
    }
}
