use super::{slot::TranslateWindowEvent, spawn_translate_window_events, Slot, SlotPlug, SlotTag};
use async_object::{Keeper, Tag};
use async_trait::async_trait;
use futures::task::Spawn;
use windows::{
    Foundation::Numerics::Vector2,
    UI::Composition::{Compositor, ContainerVisual},
};
use winit::event::WindowEvent;

struct LayerStackImpl {
    slots: Vec<Slot>,
    compositor: Compositor,
    visual: ContainerVisual,
    _slot_plug: SlotPlug,
}

impl LayerStackImpl {
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
        let slot_keeper = Slot::new(container)?;
        let slot = slot_keeper.tag();
        self.slots.push(slot_keeper);
        Ok(slot)
    }

    fn remove_layer(&mut self, slot: SlotTag) -> crate::Result<()> {
        if let Some(index) = self.slots.iter().position(|v| v.tag() == slot) {
            let slot = self.slots.remove(index);
            self.visual.Children()?.Remove(slot.container()?)?;
        }
        Ok(())
    }
    fn translate_event_to_all_layers(&mut self, event: WindowEvent<'static>) -> crate::Result<()> {
        for slot in &mut self.slots {
            slot.send_window_event(event.clone())?
        }
        Ok(())
    }
    fn translate_event_to_top_layer(&mut self, event: WindowEvent<'static>) -> crate::Result<()> {
        if let Some(slot) = self.slots.first_mut() {
            slot.send_window_event(event)?
        }
        Ok(())
    }
    fn translate_window_event(&mut self, event: WindowEvent<'static>) -> crate::Result<()> {
        match event {
            WindowEvent::Resized(size) => {
                self.visual.SetSize(Vector2 {
                    X: size.width as f32,
                    Y: size.height as f32,
                })?;
                self.translate_event_to_all_layers(event)
            }
            event @ WindowEvent::CursorMoved { .. } => self.translate_event_to_top_layer(event),
            event @ WindowEvent::MouseInput { .. } => self.translate_event_to_top_layer(event),
            _ => self.translate_event_to_all_layers(event),
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

pub struct LayerStack(Keeper<LayerStackImpl>);

impl LayerStack {
    pub fn new(
        spawner: impl Spawn,
        compositor: &Compositor,
        slot: &SlotTag,
    ) -> crate::Result<Self> {
        let frame = LayerStackImpl::new(compositor, slot)?;
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
pub struct TLayerStack(Tag<LayerStackImpl>);

impl TLayerStack {
    pub async fn add_layer(&self) -> crate::Result<Option<SlotTag>> {
        self.0.async_call_mut(|v| v.add_layer()).await.transpose()
    }
    pub async fn remove_layer(&self, slot: SlotTag) -> crate::Result<Option<()>> {
        self.0
            .async_call_mut(|v| v.remove_layer(slot))
            .await
            .transpose()
    }
}

#[async_trait]
impl TranslateWindowEvent for TLayerStack {
    async fn translate_window_event(
        &self,
        event: WindowEvent<'static>,
    ) -> crate::Result<Option<()>> {
        self.0
            .async_call_mut(|v| v.translate_window_event(event))
            .await
            .transpose()
    }
}
