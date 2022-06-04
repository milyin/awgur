use async_object::{CArc, EArc, EventBox, EventStream};
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
        let rect = compositor.CreateSpriteShapeWithGeometry(rect_geometry)?;
        rect.SetFillBrush(brush)?;
        rect.SetOffset(Vector2 { X: 0., Y: 0. })?;
        container_shape.Shapes()?.Append(rect)?;
        let shape = container_shape.into();
        Ok(shape)
    }
    fn redraw(&self) -> crate::Result<()> {
        self.container.Shapes()?.Clear()?;
        self.container
            .Shapes()?
            .Append(Self::create_background_shape(
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

#[derive(Clone, Weak)]
pub struct Background {
    container: ContainerVisual,
    core: CArc<Core>,
    events: EArc,
}

#[derive(TypedBuilder)]
pub struct BackgroundParams {
    round_corners: bool,
    color: Color,
    compositor: Compositor,
}

impl BackgroundParams {
    pub fn create(self) -> crate::Result<Background> {
        let container = self.compositor.CreateShapeVisual()?;
        let core = CArc::new(Core {
            round_corners: self.round_corners,
            color: self.color,
            compositor: self.compositor,
            container: container.clone(),
        });
        Ok(Background {
            container: container.into(),
            core,
            events: EArc::new(),
        })
    }
}

impl Background {
    pub async fn color(&self) -> Color {
        self.core.async_call(|v| v.color).await
    }
    pub async fn set_color(&mut self, color: Color) -> crate::Result<()> {
        self.core.async_call_mut(|v| v.set_color(color)).await?;
        Ok(())
    }
}

#[async_trait]
impl Panel for Background {
    fn id(&self) -> usize {
        self.core.id()
    }
    fn attach(&mut self, container: ContainerVisual) -> crate::Result<()> {
        container.Children()?.InsertAtTop(self.container.clone())?;
        Ok(())
    }
    fn detach(&mut self) -> crate::Result<()> {
        if let Ok(parent) = self.container.Parent() {
            parent.Children()?.Remove(&self.container)?;
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
            self.core.async_call_mut(|v| v.resize(*size)).await?;
        }
        self.events.send_event(event, source).await;
        Ok(())
    }
}
