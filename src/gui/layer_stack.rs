use super::{slot::TranslateWindowEvent, spawn_translate_window_events, Slot, SlotPlug};
use async_object::{run, Tag};
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
    slot_plug: SlotPlug,
}

impl LayerStackImpl {
    fn new(compositor: &Compositor, slot: &Slot) -> crate::Result<Self> {
        let visual = compositor.CreateContainerVisual()?;
        let slot_plug = slot.plug(visual.clone().into())?;
        Ok(Self {
            slots: Vec::new(),
            compositor: compositor.clone(),
            visual,
            slot_plug,
        })
    }

    fn add_layer(&mut self, pool: impl Spawn) -> crate::Result<Slot> {
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

    fn remove_layer(&mut self, slot: Slot) -> crate::Result<()> {
        if let Some(index) = self.slots.iter().position(|v| *v == slot) {
            let slot = self.slots.remove(index);
            self.visual
                .Children()?
                .Remove(slot.container_sync().unwrap())?;
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

#[derive(Clone)]
pub struct LayerStack(Tag<LayerStackImpl>);

impl LayerStack {
    pub fn new(
        spawner: impl Spawn + Clone,
        compositor: &Compositor,
        slot: &Slot,
    ) -> crate::Result<Self> {
        let layer_stack = Self(run(
            spawner.clone(),
            LayerStackImpl::new(compositor, slot)?,
        )?);
        spawn_translate_window_events(spawner, slot.clone(), layer_stack.clone())?;
        Ok(layer_stack)
    }
    pub fn add_layer_sync(&mut self, pool: impl Spawn) -> crate::Result<Option<Slot>> {
        self.0.write(|v| v.add_layer(pool)).transpose()
    }
    pub async fn add_layer(&self, pool: impl Spawn) -> crate::Result<Option<Slot>> {
        self.0.async_write(|v| v.add_layer(pool)).await.transpose()
    }
    pub async fn remove_layer(&self, slot: Slot) -> crate::Result<Option<()>> {
        self.0
            .async_write(|v| v.remove_layer(slot))
            .await
            .transpose()
    }
}

#[async_trait]
impl TranslateWindowEvent for LayerStack {
    async fn translate_window_event(
        &self,
        event: WindowEvent<'static>,
    ) -> crate::Result<Option<()>> {
        self.0
            .async_write(|v| v.translate_window_event(event))
            .await
            .transpose()
    }
    async fn name(&self) -> String {
        self.0
            .async_read(|v| v.slot_plug.slot().name())
            .await
            .unwrap_or("(dropped)".into())
    }
}
