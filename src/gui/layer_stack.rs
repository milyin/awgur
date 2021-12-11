use super::{slot::TranslateWindowEvent, spawn_translate_window_events, Slot, SlotPlug};
use async_object_derive::{async_object_decl, async_object_impl};
use async_trait::async_trait;
use futures::task::Spawn;
use windows::{
    Foundation::Numerics::Vector2,
    UI::Composition::{Compositor, ContainerVisual},
};
use winit::event::WindowEvent;

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
                self.slot_plug.slot().name(),
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
    fn translate_event_to_all_layers(&mut self, event: WindowEvent<'static>) -> crate::Result<()> {
        for slot in &mut self.slots {
            slot.send_window_event_sync(event.clone())?
        }
        Ok(())
    }
    fn translate_event_to_top_layer(&mut self, event: WindowEvent<'static>) -> crate::Result<()> {
        if let Some(slot) = self.slots.first_mut() {
            slot.send_window_event_sync(event)?
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

impl LayerStack {
    pub fn new(
        spawner: impl Spawn + Clone,
        compositor: &Compositor,
        slot: &mut Slot,
    ) -> crate::Result<Self> {
        let layer_stack = Self::create(LayerStackImpl::new(compositor, slot)?);
        spawn_translate_window_events(spawner, slot.clone(), layer_stack.downgrade())?;
        Ok(layer_stack)
    }
}

#[async_trait]
impl TranslateWindowEvent for WLayerStack {
    async fn translate_window_event(
        &mut self,
        event: WindowEvent<'static>,
    ) -> crate::Result<Option<()>> {
        self.async_translate_window_event(event).await
    }
}
