use std::sync::Arc;

use async_event_streams::{EventBox, EventStream};
use async_std::stream::StreamExt;
use async_trait::async_trait;
use futures::task::{Spawn, SpawnError, SpawnExt};

use crate::async_handle_err;

pub trait EventSource<EVT: Send + Sync + 'static> {
    fn event_stream(&self) -> EventStream<EVT>;
}

#[async_trait]
pub trait EventSink<EVT: Send + Sync + 'static> {
    // async fn on_event_own(&self, event: EVT, source: Option<Arc<EventBox>>) -> crate::Result<()>;
    async fn on_event(&self, event: &EVT, source: Option<Arc<EventBox>>) -> crate::Result<()>;
}

// #[async_trait]
// impl<EVT: Send + Sync + 'static, T: EventSink<EVT> + Send + Sync> EventSink<EVT> for Arc<T> {
//     async fn on_event(&self, event: EVT, source: Option<Arc<EventBox>>) -> crate::Result<()> {
//         self.as_ref().on_event(event, source).await
//     }
//     async fn on_event_ref(&self, event: EVT, source: Option<Arc<EventBox>>) -> crate::Result<()> {
//         self.as_ref().on_event(event, source).await
//     }
// }

// impl<EVT: Send + Sync + 'static, T: EventSource<EVT> + ?Sized> EventSource<EVT> for Box<T> {
//     fn event_stream(&self) -> EventStream<EVT> {
//         self.as_ref().event_stream()
//     }
// }

// #[async_trait]
// impl<EVT: Send + Sync + 'static, T: EventSink<EVT> + Send + Sync + ?Sized> EventSink<EVT>
//     for Box<T>
// {
//     async fn on_event(&self, event: EVT, source: Option<Arc<EventBox>>) -> crate::Result<()> {
//         self.as_ref().on_event(event, source).await
//     }
//     async fn on_event_ref(&self, event: &EVT, source: Option<Arc<EventBox>>) -> crate::Result<()> {
//         self.as_ref().on_event_ref(event, source).await
//     }
// }

#[async_trait]
pub trait EventSinkExt<EVT: Send + Sync + 'static> {
    async fn process_event_ref(&self, event: &EVT, source: Option<Arc<EventBox>>) -> crate::Result<()>;
    async fn translate_event(&self, event: EVT, source: Option<Arc<EventBox>>) -> crate::Result<()>;
    async fn translate_event_ref(&self, event: &EVT, source: Option<Arc<EventBox>>) -> crate::Result<()>;
}

#[async_trait]
impl<EVT: Send + Sync + 'static, T: EventSinkExt<EVT>> EventSink<EVT> for T {
    // async fn on_event_own(&self, event: EVT, source: Option<Arc<EventBox>>) -> crate::Result<()> {
    //     self.process_event_ref(&event, source)?;
    //     self.translate_event(event, source)?;
    //     Ok(())
    // }
    async fn on_event(&self, event: &EVT, source: Option<Arc<EventBox>>) -> crate::Result<()> {
        self.process_event_ref(event, source)?;
        self.translate_event_ref(event, source)?;
        Ok(())
    }
}

pub fn create_event_pipe<
    EVT: Send + Sync + Unpin + 'static,
    SPAWNER: Spawn,
    HANDLER: EventSink<EVT> + Send + Sync + 'static,
>(
    spawner: SPAWNER,
    source: EventStream<EVT>,
    handler: HANDLER,
) -> Result<(), SpawnError> {
    let mut source = source;
    spawner.spawn(async_handle_err(async move {
        while let Some(event) = source.next().await {
            let eventref = event.clone();
            let eventref = &*eventref;
            handler.on_event(eventref, event.into()).await?;
        }
        Ok(())
    }))
}
