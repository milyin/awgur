use std::sync::Arc;

use async_event_streams::{EventBox, EventStream, EventStreams};
use async_trait::async_trait;
use typed_builder::TypedBuilder;
use windows::{
    core::Interface,
    Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat},
    Win32::{
        Foundation::POINT, Graphics::Direct2D::ID2D1DeviceContext,
        System::WinRT::Composition::ICompositionDrawingSurfaceInterop,
    },
    UI::Composition::{
        CompositionDrawingSurface, CompositionGraphicsDevice, CompositionStretch,
        CompositionSurfaceBrush, Compositor, SpriteVisual, Visual,
    },
};

use crate::window::{check_for_device_removed, create_composition_graphics_device};

use super::{EventSink, EventSource, Panel, PanelEvent};

pub struct Surface {
    sprite_visual: SpriteVisual,
    composition_graphic_device: CompositionGraphicsDevice,
    surface: CompositionDrawingSurface,
    surface_brush: CompositionSurfaceBrush,
    panel_events: EventStreams<PanelEvent>,
}

impl Surface {
    fn new(compositor: Compositor) -> crate::Result<Self> {
        let sprite_visual = compositor.CreateSpriteVisual()?;
        let composition_graphic_device = create_composition_graphics_device(&compositor)?;
        let surface_brush = compositor.CreateSurfaceBrush()?;
        surface_brush.SetStretch(CompositionStretch::UniformToFill)?;
        let surface = composition_graphic_device.CreateDrawingSurface(
            windows::Foundation::Size::default(),
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )?;
        surface_brush.SetSurface(&surface)?;
        sprite_visual.SetBrush(&surface_brush)?;
        Ok(Self {
            sprite_visual,
            composition_graphic_device,
            surface,
            surface_brush,
            panel_events: EventStreams::new(),
        })
    }
    pub fn surface(&self) -> &CompositionDrawingSurface {
        &self.surface
    }
}

#[async_trait]
impl EventSink<PanelEvent> for Surface {
    async fn on_event(
        &self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        if let PanelEvent::Resized(size) = &event {
            self.sprite_visual.SetSize(*size)?;
        }
        self.panel_events.send_event(event, source).await;
        Ok(())
    }
}

impl EventSource<PanelEvent> for Surface {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.panel_events.create_event_stream()
    }
}

#[async_trait]
impl Panel for Surface {
    fn outer_frame(&self) -> Visual {
        self.sprite_visual.clone().into()
    }
}

#[derive(TypedBuilder)]
pub struct SurfaceParams {
    compositor: Compositor,
}

impl TryFrom<SurfaceParams> for Surface {
    type Error = crate::Error;

    fn try_from(value: SurfaceParams) -> crate::Result<Self> {
        Ok(Surface::new(value.compositor)?)
    }
}

impl TryFrom<SurfaceParams> for Arc<Surface> {
    type Error = crate::Error;

    fn try_from(value: SurfaceParams) -> crate::Result<Self> {
        Ok(Arc::new(value.try_into()?))
    }
}
