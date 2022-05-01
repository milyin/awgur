use std::hash::{Hash, Hasher};

use async_object::EventStream;
use async_trait::async_trait;

use windows::{Foundation::Numerics::Vector2, UI::Composition::ContainerVisual};
use winit::event::{ElementState, MouseButton, WindowEvent};

use super::IntoVector2;

#[derive(Clone)]
pub enum PanelEventData {
    Resized(Vector2),
    CursorMoved(Vector2),
    MouseInput {
        in_slot: bool,
        state: ElementState,
        button: MouseButton,
    },
    Empty,
}

#[derive(Clone)]
pub struct PanelEvent {
    pub source: WindowEvent<'static>,
    pub data: PanelEventData,
}

impl PanelEvent {
    pub fn from_window_event(source: WindowEvent<'static>) -> Self {
        let data = match &source {
            WindowEvent::Resized(size) => PanelEventData::Resized(size.into_vector2()),
            WindowEvent::CursorMoved { position, .. } => {
                PanelEventData::CursorMoved(position.into_vector2())
            }
            WindowEvent::MouseInput { state, button, .. } => PanelEventData::MouseInput {
                in_slot: true,
                state: *state,
                button: *button,
            },
            _ => PanelEventData::Empty,
        };
        Self { source, data }
    }

    pub fn new(source: WindowEvent<'static>, data: PanelEventData) -> Self {
        Self { source, data }
    }
}

#[async_trait]
pub trait Panel: Send + Sync {
    fn id(&self) -> usize;
    fn attach(&mut self, container: ContainerVisual) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn panel_event_stream(&self) -> EventStream<PanelEvent>;
    async fn on_panel_event(&mut self, event: PanelEvent) -> crate::Result<()>;
    fn clone_box(&self) -> Box<dyn Panel>;
}

impl Clone for Box<dyn Panel> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl Hash for Box<dyn Panel> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state)
    }
}

impl<T: Panel> PartialEq<T> for Box<dyn Panel> {
    fn eq(&self, other: &T) -> bool {
        self.id() == other.id()
    }
}
