use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

use async_event_streams::{EventBox, EventStream};
use async_trait::async_trait;
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
    fn attach(&self, container: ContainerVisual) -> crate::Result<()>;
    fn detach(&self) -> crate::Result<()>;
}

pub trait ArcPanel: Panel {
    fn id(&self) -> usize;
    fn clone_box(&self) -> Box<dyn ArcPanel>;
}

impl<EVT: Send + Sync + 'static, T: EventSource<EVT>> EventSource<EVT> for Arc<T> {
    fn event_stream(&self) -> EventStream<EVT> {
        self.as_ref().event_stream()
    }
}

#[async_trait]
impl<EVT: Send + Sync + 'static, T: EventSink<EVT> + Send + Sync> EventSink<EVT> for Arc<T> {
    async fn on_event(&self, event: EVT, source: Option<Arc<EventBox>>) -> crate::Result<()> {
        self.as_ref().on_event(event, source).await
    }
}

impl<T: Panel> Panel for Arc<T> {
    fn attach(&self, container: ContainerVisual) -> crate::Result<()> {
        self.as_ref().attach(container)
    }

    fn detach(&self) -> crate::Result<()> {
        self.as_ref().detach()
    }
}

impl<T: Panel + 'static> ArcPanel for Arc<T> {
    fn id(&self) -> usize {
        Arc::as_ptr(&self) as usize
    }
    fn clone_box(&self) -> Box<dyn ArcPanel> {
        Box::new(self.clone())
    }
}

impl Hash for dyn ArcPanel {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state)
    }
}

impl Clone for Box<dyn ArcPanel> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub fn spawn_window_event_receiver(
    pool: impl Spawn,
    panel: impl Panel + 'static,
    container: ContainerVisual,
) -> crate::Result<Sender<WindowEvent<'static>>> {
    let (tx_event_channel, mut rx_event_channel) = channel::<WindowEvent<'static>>(1024 * 64);
    let panel = panel;
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
