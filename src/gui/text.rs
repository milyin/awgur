use std::sync::Arc;

use async_event_streams::{EventBox, EventStream, EventStreams};
use async_std::sync::RwLock;
use async_trait::async_trait;
use typed_builder::TypedBuilder;
use windows::{
    core::{InParam, Interface},
    w,
    Foundation::Numerics::Vector2,
    Graphics::{
        DirectX::{DirectXAlphaMode, DirectXPixelFormat},
        SizeInt32,
    },
    Win32::{
        Foundation::{POINT, RECT},
        Graphics::{
            Direct2D::ID2D1DeviceContext,
            DirectWrite::{
                DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER,
            },
        },
        System::WinRT::Composition::ICompositionDrawingSurfaceInterop,
    },
    UI::Composition::{
        CompositionStretch, CompositionSurfaceBrush, CompositionVirtualDrawingSurface, Compositor,
        ICompositionSurface, SpriteVisual, Visual,
    },
};

use crate::window::{check_for_device_removed, composition_graphics_device, dwrite_factory};

use super::{EventSink, EventSource, Panel, PanelEvent};

struct Core {
    compositor: Compositor,
    text: String,
    surface: CompositionVirtualDrawingSurface,
    surface_brush: CompositionSurfaceBrush,
    sprite_visual: SpriteVisual,
}

impl Core {
    fn new(
        compositor: Compositor,
        sprite_visual: SpriteVisual,
        text: String,
    ) -> crate::Result<Self> {
        let surface = Self::create_surface(&compositor, &sprite_visual.Size()?)?;
        let surface_brush = compositor.CreateSurfaceBrush()?;
        surface_brush.SetStretch(CompositionStretch::None)?;
        surface_brush.SetHorizontalAlignmentRatio(0.)?;
        surface_brush.SetVerticalAlignmentRatio(0.)?;
        // surface_brush.SetTransformMatrix(Matrix3x2::translation(20., 20.))?;
        sprite_visual.SetBrush(&surface_brush)?;
        Ok(Self {
            compositor,
            text,
            surface,
            surface_brush,
            sprite_visual,
        })
    }
    fn resize(&mut self, size: Vector2) -> crate::Result<()> {
        self.init(&size)?;
        self.redraw()?;
        Ok(())
    }
    fn create_surface(
        compositor: &Compositor,
        size: &Vector2,
    ) -> crate::Result<CompositionVirtualDrawingSurface> {
        let graphic_device = composition_graphics_device(compositor)?;
        let virtual_surface = graphic_device.CreateVirtualDrawingSurface(
            SizeInt32 {
                Width: size.X as i32,
                Height: size.Y as i32,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )?;
        Ok(virtual_surface)
    }
    fn surface_interop(&self) -> crate::Result<ICompositionDrawingSurfaceInterop> {
        Ok(self.surface.cast()?)
    }
    fn init(&mut self, size: &Vector2) -> crate::Result<()> {
        self.sprite_visual.SetSize(*size)?;
        self.surface = Self::create_surface(&self.compositor, size)?;
        let surface: ICompositionSurface = self.surface_interop()?.cast()?;
        self.surface_brush.SetSurface(&surface)?;
        let dwrite_text_format = unsafe {
            dwrite_factory()?.CreateTextFormat(
                w!("Segoe UI"),
                InParam::null(),
                DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                60.,
                w!("en-US"),
            )
        }?;
        unsafe { dwrite_text_format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER) }?;
        unsafe { dwrite_text_format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER) }?;

        Ok(())
    }
    fn redraw(&self) -> crate::Result<()> {
        let size = self.sprite_visual.Size()?;
        let updaterect = RECT {
            left: 0,
            top: 0,
            right: size.X as i32,
            bottom: size.Y as i32,
        };
        let mut updateoffset = POINT { x: 0, y: 0 };

        let surface_interop = self.surface_interop()?;

        let context: Option<ID2D1DeviceContext> = check_for_device_removed(unsafe {
            surface_interop.BeginDraw(&updaterect, &mut updateoffset)
        })?;

        if let Some(context) = context {
            unsafe { surface_interop.EndDraw() }?;
        }
        // let d2d_device_context =
        Ok(())
    }
}

pub struct Text {
    visual: Visual,
    core: RwLock<Core>,
    panel_events: EventStreams<PanelEvent>,
}

impl Text {}

#[async_trait]
impl EventSink<PanelEvent> for Text {
    async fn on_event(
        &self,
        event: PanelEvent,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        if let PanelEvent::Resized(size) = &event {
            self.core.write().await.resize(*size)?;
        }
        self.panel_events.send_event(event, source).await;
        Ok(())
    }
}

impl EventSource<PanelEvent> for Text {
    fn event_stream(&self) -> EventStream<PanelEvent> {
        self.panel_events.create_event_stream()
    }
}

#[async_trait]
impl Panel for Text {
    fn outer_frame(&self) -> Visual {
        self.visual.clone()
    }
}

#[derive(TypedBuilder)]
pub struct TextParams {
    compositor: Compositor,
    text: String,
}

impl TextParams {
    pub fn create(self) -> crate::Result<Arc<Text>> {
        let sprite_visual = self.compositor.CreateSpriteVisual()?;
        let visual = sprite_visual.clone().into();
        let core = RwLock::new(Core::new(self.compositor, sprite_visual, self.text)?);
        Ok(Arc::new(Text {
            visual,
            core,
            panel_events: EventStreams::new(),
        }))
    }
}

/*
use async_object_derive::async_object_decl;
use windows::{
    Foundation::Size,
    Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat},
    UI::{
        Color, Colors,
        Composition::{
            CompositionDrawingSurface, CompositionGraphicsDevice, Compositor, SpriteVisual,
        },
    },
};

use super::{Slot, SlotPlug};

#[async_object_decl(pub Text, pub WText)]
struct TextImpl {
    compositor: Compositor,
    composition_graphics_device: CompositionGraphicsDevice,
    slot: SlotPlug,
    surface: Option<CompositionDrawingSurface>,
    visual: SpriteVisual,
    text: String,
    color: Color,
}

impl TextImpl {
    fn new(
        compositor: Compositor,
        composition_graphics_device: CompositionGraphicsDevice,
        slot: &mut Slot,
        text: String,
        color: Color,
    ) -> crate::Result<Self> {
        let visual = compositor.CreateSpriteVisual()?;
        let slot = slot.plug(visual.clone().into())?;
        Ok(Self {
            text,
            color,
            compositor,
            composition_graphics_device,
            slot,
            surface: None,
            visual,
        })
    }

    fn resize_surface(&mut self) -> crate::Result<()> {
        let size = self.visual.Size()?;
        if size.X > 0. && size.Y > 0. {
            let surface = self.composition_graphics_device.CreateDrawingSurface(
                Size {
                    Width: size.X,
                    Height: size.Y,
                },
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                DirectXAlphaMode::Premultiplied,
            )?;

            let brush = self.compositor.CreateSurfaceBrush()?;
            brush.SetSurface(surface.clone())?;
            self.surface = Some(surface);
            self.visual.SetBrush(brush)?;
        }
        Ok(())
    }

    fn redraw_text(&self) -> crate::Result<()> {
        if let Some(ref surface) = self.surface {
            let ds = CanvasComposition::CreateDrawingSession(surface)?;
            ds.Clear(Colors::Transparent()?)?;

            let size = surface.Size()?;
            let text_format = CanvasTextFormat::new()?;
            text_format.SetFontFamily("Arial")?;
            text_format.SetFontSize(size.Height / self.params.font_scale)?;
            let text: String = self.params.text.clone().into();
            let text_layout = CanvasTextLayout::Create(
                canvas_device(),
                text,
                text_format,
                size.Width,
                size.Height,
            )?;
            text_layout.SetVerticalAlignment(CanvasVerticalAlignment::Center)?;
            text_layout.SetHorizontalAlignment(CanvasHorizontalAlignment::Center)?;
            let color = if self.params.enabled {
                self.params.color.clone()
            } else {
                Colors::Gray()?
            };

            ds.DrawTextLayoutAtCoordsWithColor(text_layout, 0., 0., color)
        } else {
            Ok(())
        }
    }
}

impl Text {
    pub fn new(
        compositor: Compositor,
        composition_graphics_device: CompositionGraphicsDevice,
        slot: &mut Slot,
        text: String,
        color: Color,
    ) -> crate::Result<Self> {
        let text = Self::create(TextImpl::new(
            compositor,
            composition_graphics_device,
            slot,
            text,
            color,
        )?);
        Ok(text)
    }
}
*/
