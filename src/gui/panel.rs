use std::hash::{Hash, Hasher};

use async_trait::async_trait;

use windows::{Foundation::Numerics::Vector2, UI::Composition::ContainerVisual};
use winit::event::{ElementState, MouseButton, WindowEvent};

use super::{EventSink, EventSource, IntoVector2};

#[derive(Clone)]
pub enum PanelEvent {
    Resized(Vector2),
    CursorMoved(Vector2),
    MouseInput {
        in_slot: bool,
        state: ElementState,
        button: MouseButton,
    },
    Empty,
}

impl From<WindowEvent<'static>> for PanelEvent {
    fn from(source: WindowEvent<'static>) -> Self {
        match source {
            WindowEvent::Resized(size) => PanelEvent::Resized(size.into_vector2()),
            WindowEvent::CursorMoved { position, .. } => {
                PanelEvent::CursorMoved(position.into_vector2())
            }
            WindowEvent::MouseInput { state, button, .. } => PanelEvent::MouseInput {
                in_slot: true,
                state: state,
                button: button,
            },
            _ => PanelEvent::Empty,
        }
    }
}

#[async_trait]
pub trait Panel: Send + Sync + EventSource<PanelEvent> + EventSink<PanelEvent> {
    fn id(&self) -> usize;
    fn attach(&mut self, container: ContainerVisual) -> crate::Result<()>;
    fn detach(&mut self) -> crate::Result<()>;
    fn clone_panel(&self) -> Box<dyn Panel>;
}

impl Clone for Box<dyn Panel> {
    fn clone(&self) -> Self {
        self.clone_panel()
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
