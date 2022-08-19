mod background;
mod button;
mod layer_stack;
mod panel;
mod ribbon;
mod surface;
mod text;

use std::sync::Arc;

use async_event_streams::{EventBox, EventStream};
use async_trait::async_trait;
pub use background::{Background, BackgroundParams};
pub use button::{
    Button, ButtonEvent, ButtonParams, ButtonSkin, SimpleButtonSkin, SimpleButtonSkinParams,
};
pub use layer_stack::{LayerStack, LayerStackParams};
pub use panel::{attach, detach, spawn_window_event_receiver, ArcPanel, Panel, PanelEvent};
pub use ribbon::{CellLimit, Ribbon, RibbonOrientation, RibbonParams};
pub use surface::{Surface, SurfaceParams};
pub use text::{Text, TextParams};

use windows::Foundation::Numerics::Vector2;
use winit::dpi::{PhysicalPosition, PhysicalSize};

fn is_translated_point_in_box(point: Vector2, size: Vector2) -> bool {
    is_point_in_box(point, Vector2 { X: 0., Y: 0. }, size)
}

fn is_point_in_box(point: Vector2, offset: Vector2, size: Vector2) -> bool {
    point.X >= offset.X
        && point.X <= offset.X + size.X
        && point.Y >= offset.X
        && point.Y <= offset.Y + size.Y
}

trait IntoVector2 {
    fn into_vector2(&self) -> Vector2;
}

impl IntoVector2 for PhysicalPosition<f64> {
    fn into_vector2(&self) -> Vector2 {
        Vector2 {
            X: self.x as f32,
            Y: self.y as f32,
        }
    }
}

impl IntoVector2 for PhysicalSize<u32> {
    fn into_vector2(&self) -> Vector2 {
        Vector2 {
            X: self.width as f32,
            Y: self.height as f32,
        }
    }
}

impl IntoVector2 for Vector2 {
    fn into_vector2(&self) -> Vector2 {
        *self
    }
}

trait FromVector2<T> {
    fn from_vector2(&self) -> T;
}

impl<T> FromVector2<PhysicalPosition<f64>> for T
where
    T: IntoVector2,
{
    fn from_vector2(&self) -> PhysicalPosition<f64> {
        let v = self.into_vector2();
        PhysicalPosition {
            x: v.X as f64,
            y: v.Y as f64,
        }
    }
}

impl<T> FromVector2<PhysicalSize<u32>> for T
where
    T: IntoVector2,
{
    fn from_vector2(&self) -> PhysicalSize<u32> {
        let v = self.into_vector2();
        PhysicalSize {
            width: v.X as u32,
            height: v.Y as u32,
        }
    }
}

pub trait EventSource<EVT: Send + Sync + 'static> {
    fn event_stream(&self) -> EventStream<EVT>;
}

#[async_trait]
pub trait EventSink<EVT: Send + Sync + 'static> {
    async fn on_event(&self, event: EVT, source: Option<Arc<EventBox>>) -> crate::Result<()>;
}
