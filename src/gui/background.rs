use async_object::{CArc, EArc, EventBox, EventStream, WCArc, WEArc};
use async_trait::async_trait;
use derive_weak::Weak;
use float_ord::FloatOrd;
use std::sync::Arc;
use typed_builder::TypedBuilder;
use windows::{
    Foundation::Numerics::Vector2,
    UI::{
        Color,
        Composition::{CompositionShape, Compositor, ContainerVisual, ShapeVisual},
    },
};

use super::{EventSink, EventSource, Panel, PanelEvent};

struct Core {
    round_corners: bool,
    color: Color,
}

#[derive(Clone, Weak)]
pub struct Background {
    compositor: Compositor,
    shape: ShapeVisual,
    #[weak(WCArc)]
    core: CArc<Core>,
    #[weak(WEArc)]
    events: EArc,
}

impl Background {
    pub fn new(compositor: Compositor, color: Color, round_corners: bool) -> crate::Result<Self> {
        let shape = compositor.CreateShapeVisual()?;
        let core = CArc::new(Core {
            round_corners,
            color,
        });
        let background = Self {
            compositor,
            shape,
            core,
            events: EArc::new(),
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
        let (round_corners, color) = self.core.call(|v| (v.round_corners, v.color));
        let container_shape = self.compositor.CreateContainerShape()?;
        let rect_geometry = self.compositor.CreateRoundedRectangleGeometry()?;
        rect_geometry.SetSize(self.shape.Size()?)?;
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
        let brush = self.compositor.CreateColorBrushWithColor(color)?;
        let rect = self
            .compositor
            .CreateSpriteShapeWithGeometry(rect_geometry)?;
        rect.SetFillBrush(brush)?;
        rect.SetOffset(Vector2 { X: 0., Y: 0. })?;
        container_shape.Shapes()?.Append(rect)?;
        let shape = container_shape.into();
        Ok(shape)
    }
    pub fn color(&self) -> Color {
        self.core.call(|v| v.color)
    }
    pub fn set_color(&mut self, color: Color) -> crate::Result<()> {
        self.core.call_mut(|v| v.color = color);
        self.redraw()?;
        Ok(())
    }
    fn resize(&mut self, size: Vector2) -> crate::Result<()> {
        self.shape.SetSize(size)?;
        self.redraw()?;
        Ok(())
    }
}

#[async_trait]
impl Panel for Background {
    fn id(&self) -> usize {
        self.core.id()
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
    fn clone_panel(&self) -> Box<(dyn Panel + 'static)> {
        Box::new(self.clone())
    }
}

impl EventSource<PanelEvent> for Background {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.events.create_event_stream()
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
            self.resize(*size)?;
        }
        self.events.send_event(event, source).await;
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
