use std::sync::Arc;

use async_event_streams::{EventBox, EventSource, EventStream};
use async_trait::async_trait;
use futures::{
    channel::mpsc::{channel, Sender},
    task::{Spawn, SpawnExt},
    StreamExt,
};
use windows::{
    Foundation::Numerics::Vector2,
    UI::Composition::{ContainerVisual, Visual},
};
use winit::event::{ElementState, MouseButton, WindowEvent};

use crate::async_handle_err;

use super::{EventSink, IntoVector2};

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
    ///
    /// The visual object provided to parental panel. Position and size of this object is
    /// under control of the parent (external panel where this panel is inserted into).
    /// Usually it's ContainerVisual which includes other visuals of the panel, but it's not
    /// necessary.
    ///
    fn outer_frame(&self) -> Visual;
    fn id(&self) -> usize;
}
pub fn attach<T: Panel + ?Sized>(container: &ContainerVisual, panel: &T) -> crate::Result<()> {
    container.Children()?.InsertAtTop(&panel.outer_frame())?;
    Ok(())
}
pub fn detach(panel: &impl Panel) -> crate::Result<()> {
    // TODO: implement owner notification that panel is detached
    let visual = panel.outer_frame();
    if let Ok(parent) = visual.Parent() {
        parent.Children()?.Remove(&visual)?;
    }
    Ok(())
}

#[async_trait]
impl<EVT: Send + Sync + 'static, T: EventSink<EVT> + Send + Sync> EventSink<EVT> for Arc<T> {
    async fn on_event(&self, event: &EVT, source: Option<Arc<EventBox>>) -> crate::Result<()> {
        self.as_ref().on_event(event, source).await
    }
}

#[async_trait]
impl<EVT: Send + Sync + 'static, T: EventSink<EVT> + Send + Sync + ?Sized> EventSink<EVT>
    for Box<T>
{
    async fn on_event(&self, event: &EVT, source: Option<Arc<EventBox>>) -> crate::Result<()> {
        self.as_ref().on_event(event, source).await
    }
}

pub fn spawn_window_event_receiver(
    pool: impl Spawn,
    panel: impl Panel + 'static,
    container: ContainerVisual,
) -> crate::Result<Sender<WindowEvent<'static>>> {
    let (tx_event_channel, mut rx_event_channel) = channel::<WindowEvent<'static>>(1024 * 64);
    let panel = panel;
    attach(&container, &panel)?;
    pool.spawn(async_handle_err(async move {
        while let Some(event) = rx_event_channel.next().await {
            let panel_event = event.into();
            match &panel_event {
                // TODO: handle quit here
                PanelEvent::Resized(size) => container.SetSize(*size)?,
                _ => (),
            };
            panel.on_event(&panel_event, None).await?;
        }
        Ok(())
    }))?;
    Ok(tx_event_channel)
}
