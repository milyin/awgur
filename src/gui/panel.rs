use std::hash::{Hash, Hasher};

use futures::{
    channel::mpsc::{channel, Sender},
    task::{Spawn, SpawnExt},
    StreamExt,
};
use windows::{Foundation::Numerics::Vector2, UI::Composition::ContainerVisual};
use winit::event::{ElementState, MouseButton, WindowEvent};

use crate::async_handle_err;

use super::{EventSink, EventSource, IntoVector2};

#[derive(Clone, Debug)]
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

pub fn spawn_window_event_receiver(
    pool: impl Spawn,
    panel: impl Panel + 'static,
    container: ContainerVisual,
) -> crate::Result<Sender<WindowEvent<'static>>> {
    let (tx_event_channel, mut rx_event_channel) = channel::<WindowEvent<'static>>(1024 * 64);
    let mut panel = panel;
    panel.attach(container.clone())?;
    pool.spawn(async_handle_err(async move {
        while let Some(event) = rx_event_channel.next().await {
            let panel_event = event.into();
            match &panel_event {
                // TODO: handle quit here
                PanelEvent::Resized(size) => container.SetSize(size)?,
                _ => (),
            };
            panel.on_event(panel_event, None).await?;
        }
        Ok(())
    }))?;
    Ok(tx_event_channel)
}
