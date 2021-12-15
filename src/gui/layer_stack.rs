use super::{
    slot::{SlotEventData, SlotEventSource},
    Slot, SlotEvent, SlotPlug,
};
use crate::async_handle_err;
use async_object::Event;
use async_object_derive::{async_object_decl, async_object_impl};
use futures::{
    task::{Spawn, SpawnExt},
    StreamExt,
};
use windows::UI::Composition::{Compositor, ContainerVisual};

#[async_object_decl(pub LayerStack, pub WLayerStack)]
struct LayerStackImpl {
    slots: Vec<Slot>,
    compositor: Compositor,
    visual: ContainerVisual,
    slot_plug: SlotPlug,
}

impl LayerStackImpl {
    fn new(compositor: &Compositor, slot: &mut Slot) -> crate::Result<Self> {
        let visual = compositor.CreateContainerVisual()?;
        let slot_plug = slot.plug(visual.clone().into())?;
        Ok(Self {
            slots: Vec::new(),
            compositor: compositor.clone(),
            visual,
            slot_plug,
        })
    }
}

impl LayerStackImpl {
    fn translate_event_to_all_layers(&mut self, event: Event<SlotEvent>) -> crate::Result<()> {
        for slot in &mut self.slots {
            let data = event.as_ref().data.clone();
            slot.send_slot_event(SlotEvent::new(
                SlotEventSource::SlotEvent(event.clone()),
                data.clone(),
            ))?;
        }
        Ok(())
    }
    fn translate_event_to_top_layer(&mut self, event: Event<SlotEvent>) -> crate::Result<()> {
        if let Some(slot) = self.slots.first_mut() {
            let data = event.as_ref().data.clone();
            slot.send_slot_event(SlotEvent::new(
                SlotEventSource::SlotEvent(event.clone()),
                data.clone(),
            ))?;
        }
        Ok(())
    }
}

#[async_object_impl(LayerStack, WLayerStack)]
impl LayerStackImpl {
    pub fn add_layer(&mut self, pool: impl Spawn) -> crate::Result<Slot> {
        let container = self.compositor.CreateContainerVisual()?;
        container.SetSize(self.visual.Size()?)?;
        self.visual.Children()?.InsertAtTop(container.clone())?;
        let slot = Slot::new(
            pool,
            container,
            format!(
                "{}/LayerStack_{}",
                self.slot_plug
                    .slot()
                    .name()
                    .unwrap_or("(dropped)".to_string()),
                self.slots.len() + 1
            ),
        )?;
        self.slots.push(slot.clone());
        Ok(slot)
    }
    pub fn remove_layer(&mut self, slot: Slot) -> crate::Result<()> {
        if let Some(index) = self.slots.iter().position(|v| *v == slot) {
            let slot = self.slots.remove(index);
            self.visual.Children()?.Remove(slot.container())?;
        }
        Ok(())
    }
    fn translate_slot_event(&mut self, event: Event<SlotEvent>) -> crate::Result<()> {
        match event.as_ref().data {
            SlotEventData::Resized(size) => {
                self.visual.SetSize(size)?;
                self.translate_event_to_all_layers(event)
            }
            SlotEventData::MouseInput => self.translate_event_to_top_layer(event),
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

impl LayerStack {
    pub fn new(
        spawner: impl Spawn + Clone,
        compositor: &Compositor,
        slot: &mut Slot,
    ) -> crate::Result<Self> {
        let layer_stack = Self::create(LayerStackImpl::new(compositor, slot)?);
        let future = {
            let mut stream = slot.create_slot_event_stream();
            let mut layer_stack = layer_stack.downgrade();
            async move {
                while let Some(event) = stream.next().await {
                    if layer_stack
                        .async_translate_slot_event(event)
                        .await?
                        .is_none()
                    {
                        break;
                    }
                }
                Ok(())
            }
        };
        spawner.spawn(async_handle_err(future))?;
        Ok(layer_stack)
    }
}
