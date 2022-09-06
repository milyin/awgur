use std::borrow::Cow;

use async_event_streams::{
    EventBox, EventSink, EventSinkExt, EventSource, EventStream, EventStreams,
};
use async_event_streams_derive::{self, EventSink};
use async_std::sync::{Arc, RwLock};
use async_trait::async_trait;
use float_ord::FloatOrd;
use typed_builder::TypedBuilder;
use windows::{
    Foundation::Numerics::Vector2,
    UI::{
        Color,
        Composition::{CompositionShape, Compositor, ContainerVisual, ShapeVisual, Visual},
    },
};

use super::{Panel, PanelEvent};

struct Core {
    round_corners: bool,
    color: Color,
    compositor: Compositor,
    container: ShapeVisual,
}

impl Core {
    fn create_background_shape(
        compositor: &Compositor,
        size: Vector2,
        round_corners: bool,
        color: Color,
    ) -> crate::Result<CompositionShape> {
        let container_shape = compositor.CreateContainerShape()?;
        let rect_geometry = compositor.CreateRoundedRectangleGeometry()?;
        rect_geometry.SetSize(size)?;
        if round_corners {
            let size = rect_geometry.Size()?;
            let radius = std::cmp::min(FloatOrd(size.X), FloatOrd(size.Y)).0 / 20.;
            rect_geometry.SetCornerRadius(Vector2 {
                X: radius,
                Y: radius,
            })?;
        } else {
            rect_geometry.SetCornerRadius(Vector2 { X: 0., Y: 0. })?;
        }
        let brush = compositor.CreateColorBrushWithColor(color)?;
        let rect = compositor.CreateSpriteShapeWithGeometry(&rect_geometry)?;
        rect.SetFillBrush(&brush)?;
        rect.SetOffset(Vector2 { X: 0., Y: 0. })?;
        container_shape.Shapes()?.Append(&rect)?;
        let shape = container_shape.into();
        Ok(shape)
    }
    fn redraw(&self) -> crate::Result<()> {
        self.container.Shapes()?.Clear()?;
        self.container
            .Shapes()?
            .Append(&Self::create_background_shape(
                &self.compositor,
                self.container.Size()?,
                self.round_corners,
                self.color,
            )?)?;
        Ok(())
    }
    fn resize(&mut self, size: Vector2) -> crate::Result<()> {
        self.container.SetSize(size)?;
        self.redraw()?;
        Ok(())
    }
    fn set_color(&mut self, color: Color) -> crate::Result<()> {
        self.color = color;
        self.redraw()?;
        Ok(())
    }
}

#[derive(EventSink)]
#[event_sink(event=PanelEvent)]
pub struct Background {
    container: ContainerVisual,
    core: RwLock<Core>,
    panel_events: EventStreams<PanelEvent>,
    id: Arc<()>,
}

#[derive(TypedBuilder)]
pub struct BackgroundParams {
    round_corners: bool,
    color: Color,
    compositor: Compositor,
}

impl TryFrom<BackgroundParams> for Background {
    type Error = crate::Error;

    fn try_from(value: BackgroundParams) -> crate::Result<Self> {
        let container = value.compositor.CreateShapeVisual()?;
        let core = RwLock::new(Core {
            round_corners: value.round_corners,
            color: value.color,
            compositor: value.compositor,
            container: container.clone(),
        });
        Ok(Background {
            container: container.into(),
            core,
            panel_events: EventStreams::new(),
            id: Arc::new(()),
        })
    }
}

impl TryFrom<BackgroundParams> for Arc<Background> {
    type Error = crate::Error;

    fn try_from(value: BackgroundParams) -> crate::Result<Self> {
        Ok(Arc::new(value.try_into()?))
    }
}

impl Background {
    pub async fn color(&self) -> Color {
        self.core.read().await.color
    }
    pub async fn set_color(&self, color: Color) -> crate::Result<()> {
        self.core.write().await.set_color(color)?;
        Ok(())
    }
}

#[async_trait]
impl Panel for Background {
    fn outer_frame(&self) -> Visual {
        self.container.clone().into()
    }
    fn id(&self) -> usize {
        Arc::as_ptr(&self.id) as usize
    }
}

impl EventSource<PanelEvent> for Background {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.panel_events.create_event_stream()
    }
}

#[async_trait]
impl EventSinkExt<PanelEvent> for Background {
    type Error = crate::Error;
    async fn on_event<'a>(
        &'a self,
        event: Cow<'a, PanelEvent>,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        if let PanelEvent::Resized(size) = event.as_ref() {
            self.core.write().await.resize(*size)?;
        }
        self.panel_events
            .send_event(event.into_owned(), source)
            .await;
        Ok(())
    }
}
