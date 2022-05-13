use std::sync::Arc;
use async_object::{EventBox, EventStream};
use async_object_derive::{async_object_impl, async_object_with_events_decl};
use async_trait::async_trait;
use float_ord::FloatOrd;
use typed_builder::TypedBuilder;
use windows::{
    Foundation::Numerics::Vector2,
    UI::{
        Color,
        Composition::{CompositionShape, Compositor, ContainerVisual, ShapeVisual},
    },
};

use super::{EventSink, EventSource, Panel, PanelEvent};

#[async_object_with_events_decl(pub Background, pub WBackground)]
pub struct BackgroundImpl {
    compositor: Compositor,
    shape: ShapeVisual,
    round_corners: bool,
    color: Color,
}

impl BackgroundImpl {
    fn new(compositor: Compositor, color: Color, round_corners: bool) -> crate::Result<Self> {
        let shape = compositor.CreateShapeVisual()?;
        let background = Self {
            compositor,
            shape,
            color,
            round_corners,
        };
        background.redraw()?;
        Ok(background)
    }
    fn redraw(&self) -> crate::Result<()> {
        self.shape.Shapes()?.Clear()?;
        self.shape
            .Shapes()?
            .Append(self.create_background_shape()?)?;
        Ok(())
    }
    fn create_background_shape(&self) -> crate::Result<CompositionShape> {
        let container_shape = self.compositor.CreateContainerShape()?;
        let rect_geometry = self.compositor.CreateRoundedRectangleGeometry()?;
        rect_geometry.SetSize(self.shape.Size()?)?;
        if self.round_corners {
            let size = rect_geometry.Size()?;
            let radius = std::cmp::min(FloatOrd(size.X), FloatOrd(size.Y)).0 / 20.;
            rect_geometry.SetCornerRadius(Vector2 {
                X: radius,
                Y: radius,
            })?;
        } else {
            rect_geometry.SetCornerRadius(Vector2 { X: 0., Y: 0. })?;
        }
        let brush = self
            .compositor
            .CreateColorBrushWithColor(self.color.clone())?;
        let rect = self
            .compositor
            .CreateSpriteShapeWithGeometry(rect_geometry)?;
        rect.SetFillBrush(brush)?;
        rect.SetOffset(Vector2 { X: 0., Y: 0. })?;
        container_shape.Shapes()?.Append(rect)?;
        let shape = container_shape.into();
        Ok(shape)
    }
}

#[async_object_impl(Background, WBackground)]
impl BackgroundImpl {
    pub fn set_color(&mut self, color: Color) -> crate::Result<()> {
        self.color = color;
        self.redraw()?;
        Ok(())
    }

    pub fn round_corners(&self) -> bool {
        self.round_corners
    }
    pub fn color(&self) -> Color {
        self.color
    }

    fn resize(&mut self, size: Vector2) -> crate::Result<()> {
        self.shape.SetSize(size)?;
        self.redraw()?;
        Ok(())
    }
    fn attach(&mut self, container: ContainerVisual) -> crate::Result<()> {
        container.Children()?.InsertAtTop(self.shape.clone())?;
        Ok(())
    }
    fn detach(&mut self) -> crate::Result<()> {
        if let Ok(parent) = self.shape.Parent() {
            parent.Children()?.Remove(&self.shape)?;
        }
        Ok(())
    }
}

impl Background {
    pub fn new(compositor: Compositor, color: Color, round_corners: bool) -> crate::Result<Self> {
        let background = Self::create(BackgroundImpl::new(compositor, color, round_corners)?);
        Ok(background)
    }
}

#[async_trait]
impl Panel for Background {
    fn id(&self) -> usize {
        self.id()
    }
    fn attach(&mut self, container: ContainerVisual) -> crate::Result<()> {
        self.attach(container)
    }
    fn detach(&mut self) -> crate::Result<()> {
        self.detach()
    }
    fn clone_panel(&self) -> Box<(dyn Panel + 'static)> {
        Box::new(self.clone())
    }
}

impl EventSource<PanelEvent> for Background {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.create_event_stream()
    }
}

#[async_trait]
impl EventSink<PanelEvent> for Background {
    async fn on_event(
        &mut self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        if let PanelEvent::Resized(size) = &event {
            self.async_resize(*size).await?;
        }
        self.send_event(event, source).await;
        Ok(())
    }
}

#[derive(TypedBuilder)]
pub struct BackgroundBuilder {
    round_corners: bool,
    color: Color,
}

impl BackgroundBuilder {
    pub fn new(self, compositor: Compositor) -> crate::Result<Background> {
        Background::new(compositor, self.color, self.round_corners)
    }
}
