use async_std::sync::{Arc, RwLock};

use super::{ArcPanel, EventSink, EventSource, Panel, PanelEvent};
use async_events::{EventBox, EventQueues, EventStream};
use async_trait::async_trait;

use typed_builder::TypedBuilder;
use windows::UI::Composition::{Compositor, ContainerVisual};

struct Core {
    layers: Vec<Box<dyn ArcPanel>>,
}

pub struct LayerStack {
    container: ContainerVisual,
    core: RwLock<Core>,
    events: EventQueues,
}

impl LayerStack {
    async fn layers(&self) -> Vec<Box<dyn ArcPanel>> {
        self.core.read().await.layers.clone()
    }

    pub async fn push_panel(&mut self, panel: impl ArcPanel) -> crate::Result<()> {
        panel.attach(self.container.clone())?;
        self.core.write().await.layers.push(panel.clone_box());
        Ok(())
    }

    pub async fn remove_panel(&mut self, panel: impl ArcPanel) -> crate::Result<()> {
        let mut core = self.core.write().await;
        if let Some(index) = core.layers.iter().position(|v| v.id() == panel.id()) {
            panel.detach()?;
            core.layers.remove(index);
        }
        Ok(())
    }
    async fn translate_event_to_all_layers(
        &self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        // TODO: run simultaneously
        for item in self.layers().await {
            item.on_event(event.clone(), source.clone()).await?;
        }
        Ok(())
    }
    async fn translate_event_to_top_layer(
        &self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        if let Some(item) = self.layers().await.first_mut() {
            item.on_event(event, source).await?;
        }
        Ok(())
    }
    async fn translate_event(
        &self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        match event {
            PanelEvent::Resized(size) => {
                self.container.SetSize(size)?;
                self.translate_event_to_all_layers(event, source).await
            }
            PanelEvent::MouseInput { .. } => self.translate_event_to_top_layer(event, source).await,
            _ => self.translate_event_to_all_layers(event, source).await,
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

#[derive(TypedBuilder)]
pub struct LayerStackParams {
    compositor: Compositor,
    #[builder(default)]
    layers: Vec<Box<dyn ArcPanel>>,
}

impl LayerStackParams {
    pub fn push_panel(mut self, panel: impl ArcPanel) -> Self {
        self.layers.push(panel.clone_box());
        self
    }
    pub fn create(self) -> crate::Result<LayerStack> {
        let mut layers = self.layers;
        let container = self.compositor.CreateContainerVisual()?;
        for layer in &mut layers {
            layer.attach(container.clone())?;
        }
        let core = RwLock::new(Core { layers });
        // container.SetComment(HSTRING::from("LAYER_STACK"))?;
        Ok(LayerStack {
            container,
            core,
            events: EventQueues::new(),
        })
    }
}

impl Panel for LayerStack {
    fn attach(&self, container: ContainerVisual) -> crate::Result<()> {
        container.Children()?.InsertAtTop(self.container.clone())?;
        Ok(())
    }
    fn detach(&self) -> crate::Result<()> {
        if let Ok(parent) = self.container.Parent() {
            parent.Children()?.Remove(&self.container.clone())?;
        }
        Ok(())
    }
}

impl EventSource<PanelEvent> for LayerStack {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.events.create_event_stream()
    }
}

#[async_trait]
impl EventSink<PanelEvent> for LayerStack {
    async fn on_event(
        &self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        self.translate_event(event.clone(), source.clone()).await?;
        self.events.send_event(event, source).await;
        Ok(())
    }
}
