use async_object_derive::{async_object_decl, async_object_impl};
use float_ord::FloatOrd;
use futures::{
    task::{Spawn, SpawnExt},
    StreamExt,
};
use windows::{
    Foundation::Numerics::Vector2,
    UI::{
        Color,
        Composition::{CompositionShape, Compositor, ShapeVisual},
    },
};

use crate::gui::Slot;
use crate::gui::SlotPlug;
use crate::gui::WSlot;
use crate::unwrap_err;

#[async_object_decl(pub Background, pub WBackground)]
pub struct BackgroundIimpl {
    compositor: Compositor,
    slot: SlotPlug,
    shape: ShapeVisual,
    round_corners: bool,
    color: Color,
}

impl BackgroundIimpl {
    fn new(
        compositor: &Compositor,
        mut slot: Slot,
        color: Color,
        round_corners: bool,
    ) -> crate::Result<Self> {
        let compositor = compositor.clone();
        let shape = compositor.CreateShapeVisual()?;
        let slot = slot.plug(shape.clone().into())?;
        let background = Self {
            compositor,
            slot,
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
impl BackgroundIimpl {
    pub fn set_color(&mut self, color: Color) -> crate::Result<()> {
        self.color = color;
        self.redraw()?;
        Ok(())
    }

    pub fn set_size(&mut self, size: Vector2) -> crate::Result<()> {
        self.shape.SetSize(size)?;
        self.redraw()?;
        Ok(())
    }

    pub fn round_corners(&self) -> bool {
        self.round_corners
    }
    pub fn color(&self) -> Color {
        self.color
    }

    fn slot(&self) -> WSlot {
        self.slot.slot()
    }
}

impl Background {
    pub fn new(
        spawner: impl Spawn + Clone,
        compositor: &Compositor,
        slot: Slot,
        color: Color,
        round_corners: bool,
    ) -> crate::Result<Self> {
        let background = Self::create(BackgroundIimpl::new(
            compositor,
            slot,
            color,
            round_corners,
        )?);
        background.clone().spawn_event_handlers(spawner)?;
        Ok(background)
    }
    fn spawn_event_handlers(self, spawner: impl Spawn) -> crate::Result<()> {
        let mut backgorund = self.clone();
        let slot = self.slot();
        let func = unwrap_err(async move {
            if let Some(mut stream) = slot.upgrade().map(|v| v.on_slot_resized()) {
                while let Some(size) = stream.next().await {
                    backgorund.async_set_size(size.as_ref().0).await?;
                }
            }
            Ok(())
        });
        spawner.spawn(func)?;
        Ok(())
    }
}
