use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use async_object::{Keeper, Tag};
use windows::UI::Composition::{Compositor, ContainerVisual};

use crate::event::{MouseLeftPressed, MouseLeftPressedFocused, SendSlotEvent, SlotSize};

use super::{SlotKeeper, SlotTag};

pub struct Frame {
    slots: Vec<SlotKeeper>,
    compositor: Compositor,
    root_visual: ContainerVisual,
}

impl Frame {
    fn new(compositor: Compositor, root_visual: ContainerVisual) -> crate::Result<Self> {
        Ok(Self {
            slots: Vec::new(),
            compositor,
            root_visual,
        })
    }

    fn open_slot(&mut self) -> crate::Result<SlotTag> {
        let container = self.compositor.CreateContainerVisual()?;
        container.SetSize(self.root_visual.Size()?)?;
        self.root_visual
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
            self.root_visual.Children()?.Remove(slot.container()?)?;
        }
        Ok(())
    }
}

impl SendSlotEvent for Frame {
    fn send_size(&mut self, size: SlotSize) -> crate::Result<()> {
        self.root_visual.SetSize(size.0)?;
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
pub struct KFrame(Keeper<Frame>);

impl KFrame {
    pub fn new(compositor: Compositor, root_visual: ContainerVisual) -> crate::Result<Self> {
        let frame = Frame::new(compositor, root_visual)?;
        let keeper = Keeper::new(frame);
        Ok(Self(keeper))
    }
    pub fn tag(&self) -> TFrame {
        TFrame(self.0.tag())
    }
    pub fn get(&self) -> RwLockReadGuard<'_, Frame> {
        self.0.get()
    }
    pub fn get_mut(&self) -> RwLockWriteGuard<'_, Frame> {
        self.0.get_mut()
    }
}

#[derive(Clone, PartialEq)]
pub struct TFrame(Tag<Frame>);

impl TFrame {
    pub fn open_slot(&self) -> crate::Result<SlotTag> {
        self.0.call_mut(|frame| frame.open_slot())?
    }
    pub fn close_slot(&self, slot: SlotTag) -> crate::Result<()> {
        self.0.call_mut(|frame| frame.close_slot(slot))?
    }
}

impl SendSlotEvent for TFrame {
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
