use super::{EventSink, EventSource, Panel, PanelEvent, PanelEventData};
use async_object::EventStream;
use async_object_derive::{async_object_impl, async_object_with_events_decl};
use async_trait::async_trait;

use windows::{
    core::HSTRING,
    UI::Composition::{Compositor, ContainerVisual},
};

#[async_object_with_events_decl(pub LayerStack, pub WLayerStack)]
struct LayerStackImpl {
    layers: Vec<Box<dyn Panel>>,
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
    fn layers(&self) -> Vec<Box<dyn Panel>> {
        self.layers.clone()
    }
    fn visual(&self) -> ContainerVisual {
        self.container.clone()
    }
}

impl LayerStack {
    async fn translate_event_to_all_layers(&mut self, event: PanelEvent) -> crate::Result<()> {
        // TODO: run simultaneously
        for mut item in self.layers() {
            item.on_event(event.clone()).await?;
        }
        Ok(())
    }
    async fn translate_event_to_top_layer(&mut self, event: PanelEvent) -> crate::Result<()> {
        if let Some(item) = self.async_layers().await.first_mut() {
            item.on_event(event).await?;
        }
        Ok(())
    }
    pub async fn translate_event(&mut self, event: PanelEvent) -> crate::Result<()> {
        match event.data {
            PanelEventData::Resized(size) => {
                self.async_visual().await.SetSize(size)?;
                self.translate_event_to_all_layers(event).await
            }
            PanelEventData::MouseInput { .. } => self.translate_event_to_top_layer(event).await,
            _ => self.translate_event_to_all_layers(event).await,
        }
    }
}

#[async_object_impl(LayerStack, WLayerStack)]
impl LayerStackImpl {
    pub fn push_panel(&mut self, mut panel: impl Panel + 'static) -> crate::Result<()> {
        panel.attach(self.container.clone())?;
        self.layers.push(Box::new(panel));
        Ok(())
    }
    pub fn remove_panel(&mut self, mut panel: impl Panel) -> crate::Result<()> {
        if let Some(index) = self.layers.iter().position(|v| *v == panel) {
            panel.detach()?;
            self.layers.remove(index);
        }
        Ok(())
    }
    fn attach(&mut self, container: ContainerVisual) -> crate::Result<()> {
        container.Children()?.InsertAtTop(self.container.clone())?;
        Ok(())
    }
    fn detach(&mut self) -> crate::Result<()> {
        if let Ok(parent) = self.container.Parent() {
            parent.Children()?.Remove(&self.container.clone())?;
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
        container.SetComment(HSTRING::from("LAYER_STACK"))?;
        let layer_stack = Self::create(LayerStackImpl::new(container));
        Ok(layer_stack)
    }
}

impl Panel for LayerStack {
    fn id(&self) -> usize {
        self.id()
    }
    fn attach(&mut self, container: ContainerVisual) -> crate::Result<()> {
        self.attach(container)
    }
    fn detach(&mut self) -> crate::Result<()> {
        self.detach()
    }
    fn clone_panel(&self) -> Box<(dyn Panel + 'static)> {
        Box::new(self.clone())
    }
}

impl EventSource<PanelEvent> for LayerStack {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.create_event_stream()
    }
}

#[async_trait]
impl EventSink<PanelEvent> for LayerStack {
    async fn on_event(&mut self, event: PanelEvent) -> crate::Result<()> {
        self.translate_event(event.clone()).await?;
        self.send_event(event).await;
        Ok(())
    }
}
