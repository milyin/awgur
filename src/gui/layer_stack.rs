use super::{
    slot::TranslateWindowEvent, spawn_translate_window_events, SlotKeeper, SlotPlug, SlotTag,
};
use async_object::{Keeper, Tag};
use async_trait::async_trait;
use futures::task::Spawn;
use windows::{
    Foundation::Numerics::Vector2,
    UI::Composition::{Compositor, ContainerVisual},
};
use winit::{dpi::PhysicalSize, event::WindowEvent};

pub struct LayerStack {
    slots: Vec<SlotKeeper>,
    compositor: Compositor,
    visual: ContainerVisual,
    _slot_plug: SlotPlug,
}

impl LayerStack {
    fn new(compositor: &Compositor, slot: &SlotTag) -> crate::Result<Self> {
        let visual = compositor.CreateContainerVisual()?;
        let _slot_plug = slot.plug(visual.clone().into())?;
        Ok(Self {
            slots: Vec::new(),
            compositor: compositor.clone(),
            visual,
            _slot_plug,
        })
    }

    fn add_layer(&mut self) -> crate::Result<SlotTag> {
        let container = self.compositor.CreateContainerVisual()?;
        container.SetSize(self.visual.Size()?)?;
        self.visual.Children()?.InsertAtTop(container.clone())?;
        let slot_keeper = SlotKeeper::new(container)?;
        let slot = slot_keeper.tag();
        self.slots.push(slot_keeper);
        Ok(slot)
    }

    pub fn remove_layer(&mut self, slot: SlotTag) -> crate::Result<()> {
        if let Some(index) = self.slots.iter().position(|v| v.tag() == slot) {
            let slot = self.slots.remove(index);
            self.visual.Children()?.Remove(slot.container()?)?;
        }
        Ok(())
    }

    fn translate_window_event_resized(&mut self, size: PhysicalSize<u32>) -> crate::Result<()> {
        self.visual.SetSize(Vector2 {
            X: size.width as f32,
            Y: size.height as f32,
        })?;
        for slot in &mut self.slots {
            slot.translate_window_event(WindowEvent::Resized(size))?
        }
        Ok(())
    }

    fn translate_window_event(&mut self, event: WindowEvent) -> crate::Result<()> {
        match event {
            WindowEvent::Resized(size) => self.translate_window_event_resized(size),
            _ => Ok(()),
        }
    }
}
// fn send_mouse_left_pressed(&mut self, event: MouseLeftPressed) -> crate::Result<()> {
//     for slot in &mut self.slots {
//         slot.send_mouse_left_pressed(event.clone())?;
//     }
//     Ok(())
// }

// fn send_mouse_left_pressed_focused(
//     &mut self,
//     event: MouseLeftPressedFocused,
// ) -> crate::Result<()> {
//     if let Some(slot) = self.slots.last_mut() {
//         slot.send_mouse_left_pressed_focused(event)?;
//     }
//     Ok(())
// }

pub struct KLayerStack(Keeper<LayerStack>);

impl KLayerStack {
    pub fn new(
        spawner: impl Spawn,
        compositor: &Compositor,
        slot: &SlotTag,
    ) -> crate::Result<Self> {
        let frame = LayerStack::new(compositor, slot)?;
        let keeper = Self(Keeper::new(frame));
        spawn_translate_window_events(spawner, slot.clone(), keeper.tag())?;
        Ok(keeper)
    }
    pub fn tag(&self) -> TLayerStack {
        TLayerStack(self.0.tag())
    }
    pub fn add_layer(&mut self) -> crate::Result<SlotTag> {
        self.0.get_mut().add_layer()
    }
}

#[derive(Clone, PartialEq)]
pub struct TLayerStack(Tag<LayerStack>);

impl TLayerStack {
    pub async fn add_layer(&self) -> crate::Result<SlotTag> {
        self.0.async_call_mut(|v| v.add_layer()).await?
    }
    pub async fn remove_layer(&self, slot: SlotTag) -> crate::Result<()> {
        self.0.async_call_mut(|v| v.remove_layer(slot)).await?
    }
}

#[async_trait]
impl TranslateWindowEvent for TLayerStack {
    async fn translate_window_event(&self, event: WindowEvent<'static>) -> crate::Result<()> {
        self.0
            .async_call_mut(|v| v.translate_window_event(event))
            .await?
    }
}
