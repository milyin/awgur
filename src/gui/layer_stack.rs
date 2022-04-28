use super::{Plug, SlotEvent, SlotEventData};
use async_object_derive::{async_object_impl, async_object_with_events_decl};

use windows::UI::Composition::{Compositor, ContainerVisual};

#[async_object_with_events_decl(pub LayerStack, pub WLayerStack)]
struct LayerStackImpl {
    layers: Vec<Box<dyn Plug>>,
    container: ContainerVisual,
}

impl LayerStackImpl {
    fn new(container: ContainerVisual) -> Self {
        Self {
            layers: Vec::new(),
            container,
        }
    }
}

#[async_object_impl(LayerStack, WLayerStack)]
impl LayerStackImpl {
    fn layers(&self) -> Vec<Box<dyn Plug>> {
        self.layers.clone()
    }
    fn visual(&self) -> ContainerVisual {
        self.container.clone()
    }
}

impl LayerStack {
    async fn translate_event_to_all_layers(&mut self, event: SlotEvent) -> crate::Result<()> {
        for mut item in self.layers() {
            item.on_slot_event(event.clone()).await?;
        }
        Ok(())
    }
    async fn translate_event_to_top_layer(&mut self, event: SlotEvent) -> crate::Result<()> {
        if let Some(item) = self.async_layers().await.first_mut() {
            item.on_slot_event(event).await?;
        }
        Ok(())
    }
    pub async fn translate_slot_event(&mut self, event: SlotEvent) -> crate::Result<()> {
        match event.data {
            SlotEventData::Resized(size) => {
                self.async_visual().await.SetSize(size)?;
                self.translate_event_to_all_layers(event).await
            }
            SlotEventData::MouseInput { .. } => self.translate_event_to_top_layer(event).await,
            _ => self.translate_event_to_all_layers(event).await,
        }
    }
}

#[async_object_impl(LayerStack, WLayerStack)]
impl LayerStackImpl {
    pub fn add_layer(&mut self, item: impl Plug + 'static) -> crate::Result<()> {
        let visual = item.get_visual();
        visual.SetSize(self.container.Size()?)?;
        self.container.Children()?.InsertAtTop(visual)?;
        self.layers.push(Box::new(item));
        Ok(())
    }
    pub fn remove_layer(&mut self, item: impl Plug) -> crate::Result<()> {
        if let Some(index) = self.layers.iter().position(|v| *v == item) {
            self.container.Children()?.Remove(item.get_visual())?;
            self.layers.remove(index);
        }
        Ok(())
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
    pub fn new(compositor: Compositor) -> crate::Result<Self> {
        let container = compositor.CreateContainerVisual()?;
        let layer_stack = Self::create(LayerStackImpl::new(container));
        Ok(layer_stack)
    }
}
