use async_object_derive::{async_object_decl, async_object_impl};
use float_ord::FloatOrd;
use futures::{
    task::{Spawn, SpawnExt},
    StreamExt,
};
use typed_builder::TypedBuilder;
use windows::{
    Foundation::Numerics::Vector2,
    UI::{
        Color,
        Composition::{CompositionShape, Compositor, ShapeVisual},
    },
};

use crate::async_handle_err;
use crate::gui::Slot;
use crate::gui::SlotPlug;
use crate::gui::WSlot;

use super::slot::SlotEventData;

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
        slot: &mut Slot,
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

    pub fn slot(&self) -> WSlot {
        self.slot.slot()
    }
}

impl Background {
    pub fn new(
        spawner: impl Spawn + Clone,
        compositor: &Compositor,
        slot: &mut Slot,
        color: Color,
        round_corners: bool,
    ) -> crate::Result<Self> {
        let background = Self::create(BackgroundIimpl::new(
            compositor,
            slot,
            color,
            round_corners,
        )?);
        let future = async_handle_err({
            let mut stream = slot.create_slot_event_stream();
            let mut background = background.downgrade();
            async move {
                while let Some(event) = stream.next().await {
                    match event.as_ref().data {
                        SlotEventData::Resized(size) => {
                            if background.async_set_size(size).await?.is_none() {
                                break;
                            }
                        }
                        _ => (),
                    };
                }
                Ok(())
            }
        });
        spawner.spawn(future)?;
        Ok(background)
    }
}

#[derive(TypedBuilder)]
pub struct BackgroundBuilder {
    round_corners: bool,
    color: Color,
}

impl BackgroundBuilder {
    pub fn new(
        self,
        spawner: impl Spawn + Clone,
        compositor: &Compositor,
        slot: &mut Slot,
    ) -> crate::Result<Background> {
        Background::new(spawner, compositor, slot, self.color, self.round_corners)
    }
}
