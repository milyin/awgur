use async_object_derive::{async_object_impl, async_object_with_events_decl};
use futures::{
    channel::mpsc::{channel, Sender},
    task::{Spawn, SpawnExt},
    StreamExt,
};
use windows::{
    Foundation::Numerics::Vector2,
    UI::Composition::{Compositor, ContainerVisual},
};
use winit::event::WindowEvent;

use crate::async_handle_err;

use super::{Panel, PanelEvent, PanelEventData};

#[async_object_with_events_decl(pub Root, pub WRoot)]
struct RootImpl {
    panel: Option<Box<dyn Panel>>,
    root_visual: ContainerVisual,
    tx_event_channel: Sender<WindowEvent<'static>>,
}

impl RootImpl {
    fn new(
        compositor: &Compositor,
        size: Vector2,
        tx_event_channel: Sender<WindowEvent<'static>>,
    ) -> crate::Result<Self> {
        let root_visual = compositor.CreateContainerVisual()?;
        root_visual.SetSize(size)?;
        Ok(RootImpl {
            panel: None,
            root_visual,
            tx_event_channel,
        })
    }
}

#[async_object_impl(Root, WRoot)]
impl RootImpl {
    pub fn tx_event_channel(&self) -> Sender<WindowEvent<'static>> {
        self.tx_event_channel.clone()
    }
    pub fn visual(&self) -> ContainerVisual {
        self.root_visual.clone()
    }
    pub fn set_panel(&mut self, panel: impl Panel + 'static) -> crate::Result<()> {
        if let Some(item) = self.panel.take() {
            self.root_visual.Children()?.Remove(item.get_visual())?;
        }
        self.root_visual
            .Children()?
            .InsertAtTop(panel.get_visual())?;
        self.panel = Some(Box::new(panel));
        Ok(())
    }
    fn panel(&self) -> Option<Box<dyn Panel>> {
        self.panel.clone()
    }
    fn handle_event(&self, event: &PanelEvent) -> crate::Result<()> {
        match &event.data {
            PanelEventData::Resized(size) => self.root_visual.SetSize(size)?,
            _ => (),
        };
        Ok(())
    }
}

impl Root {
    pub fn new(pool: impl Spawn, compositor: &Compositor, size: Vector2) -> crate::Result<Self> {
        let (tx_event_channel, mut rx_event_channel) = channel(1024 * 64);
        let root = RootImpl::new(compositor, size, tx_event_channel)?;
        let root = Root::create(root);
        let wroot = root.downgrade();
        pool.spawn(async_handle_err(async move {
            while let Some(event) = rx_event_channel.next().await {
                if let Some(root) = wroot.upgrade() {
                    let event = PanelEvent::from_window_event(event);
                    root.async_handle_event(&event).await?;
                    if let Some(item) = root.panel().as_mut() {
                        item.on_panel_event(event).await?
                    }
                } else {
                    break;
                }
            }
            Ok(())
        }))?;

        Ok(root)
    }
}
