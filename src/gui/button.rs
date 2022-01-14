use super::{Slot, SlotEvent, SlotEventData, SlotEventSource, SlotPlug};
use crate::async_handle_err;
use async_object::{Event, EventStream};
use async_object_derive::{async_object_impl, async_object_with_events_decl};
use futures::{
    task::{Spawn, SpawnExt},
    StreamExt,
};
use windows::UI::Composition::{Compositor, ContainerVisual};

#[derive(PartialEq)]
pub enum ButtonEvent {
    Pressed,
}

#[async_object_with_events_decl(pub Button, pub WButton)]
struct ButtonImpl {
    slot: Slot,
    _slot_plug: SlotPlug,
    visual: ContainerVisual,
}

impl ButtonImpl {
    fn new(compositor: &Compositor, slot: &mut Slot) -> crate::Result<Self> {
        let visual = compositor.CreateContainerVisual()?;
        let _slot_plug = slot.plug(visual.clone().into())?;
        let slot = Slot::new(visual.clone(), "button".into())?;
        Ok(Self {
            slot,
            _slot_plug,
            visual,
        })
    }
}

#[async_object_impl(Button, WButton)]
impl ButtonImpl {
    pub fn slot(&self) -> Slot {
        self.slot.clone()
    }
}

impl Button {
    pub fn new(
        spawner: impl Spawn,
        compositor: &Compositor,
        slot: &mut Slot,
    ) -> crate::Result<Self> {
        let button = Self::create(ButtonImpl::new(compositor, slot)?)?;
        let future = {
            let mut stream = slot.create_slot_event_stream();
            let wbutton = button.downgrade();
            async move {
                while let Some(event) = stream.next().await {
                    if let Some(mut button) = wbutton.upgrade() {
                        button.translate_slot_event(event).await?
                    } else {
                        break;
                    }
                }
                Ok(())
            }
        };
        spawner.spawn(async_handle_err(future))?;
        Ok(button)
    }

    pub fn create_button_event_stream(&self) -> EventStream<ButtonEvent> {
        self.create_event_stream()
    }

    async fn translate_slot_event(&mut self, event: Event<SlotEvent>) -> crate::Result<()> {
        let data = match &event.as_ref().data {
            SlotEventData::MouseInput => {
                self.send_event(ButtonEvent::Pressed).await;
                None
            }
            data => Some(data.clone()),
        };
        if let Some(data) = data {
            self.async_slot()
                .await
                .send_slot_event(SlotEvent::new(SlotEventSource::SlotEvent(event), data))
                .await;
        }
        Ok(())
    }
}
