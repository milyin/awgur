use std::sync::Arc;

use async_event_streams::{EventBox, EventStream, EventStreams};
use async_std::sync::RwLock;
use async_trait::async_trait;
use typed_builder::TypedBuilder;
use windows::{
    core::{InParam, Interface},
    w,
    Foundation::Numerics::{Matrix3x2, Vector2},
    Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat},
    Win32::{
        Foundation::POINT,
        Graphics::{
            Direct2D::{
                Common::{D2D1_COLOR_F, D2D_POINT_2F},
                ID2D1DeviceContext, D2D1_BRUSH_PROPERTIES, D2D1_DRAW_TEXT_OPTIONS,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            },
            DirectWrite::{
                DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER,
            },
            Gdi::CreateSolidBrush,
        },
        System::WinRT::Composition::ICompositionDrawingSurfaceInterop,
    },
    UI::Composition::{
        CompositionGraphicsDevice, CompositionStretch, CompositionSurfaceBrush, Compositor,
        SpriteVisual, Visual,
    },
};

use crate::window::{
    check_for_device_removed, create_composition_graphics_device, dwrite_factory, ToWide,
};

use super::{EventSink, EventSource, Panel, PanelEvent};

struct Core {
    composition_graphic_device: CompositionGraphicsDevice,
    text: String,
    surface_brush: CompositionSurfaceBrush,
    sprite_visual: SpriteVisual,
}

impl Core {
    fn new(
        compositor: Compositor,
        sprite_visual: SpriteVisual,
        text: String,
    ) -> crate::Result<Self> {
        let composition_graphic_device = create_composition_graphics_device(&compositor)?;
        let surface_brush = compositor.CreateSurfaceBrush()?;
        surface_brush.SetStretch(CompositionStretch::None)?;
        surface_brush.SetHorizontalAlignmentRatio(0.)?;
        surface_brush.SetVerticalAlignmentRatio(0.)?;
        // surface_brush.SetTransformMatrix(windows::Foundation::Numerics::Matrix3x2::translation(
        //     20., 20.,
        // ))?;
        sprite_visual.SetBrush(&surface_brush)?;
        Ok(Self {
            composition_graphic_device,
            text,
            surface_brush,
            sprite_visual,
        })
    }
    fn resize(&mut self, size: Vector2) -> crate::Result<()> {
        self.redraw(&size)?;
        self.sprite_visual.SetSize(size)?;
        Ok(())
    }
    fn redraw(&mut self, size: &Vector2) -> crate::Result<()> {
        let surface = self.composition_graphic_device.CreateDrawingSurface(
            windows::Foundation::Size {
                Width: size.X,
                Height: size.Y,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )?;
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
        let text_layout = unsafe {
            dwrite_factory()?.CreateTextLayout(
                self.text.as_str().to_wide().0.as_slice(),
                &dwrite_text_format,
                size.X,
                size.Y,
            )
        }?;

        let mut updateoffset = POINT { x: 0, y: 0 };
        let surface_interop: ICompositionDrawingSurfaceInterop = surface.cast()?;
        let context: Option<ID2D1DeviceContext> = check_for_device_removed(unsafe {
            surface_interop.BeginDraw(std::ptr::null(), &mut updateoffset)
        })?;
        if let Some(context) = context {
            let clearcolor = D2D1_COLOR_F {
                r: 255.,
                g: 127.,
                b: 0.,
                a: 1.,
            };
            let text_color = D2D1_COLOR_F {
                r: 0.,
                g: 0.,
                b: 0.,
                a: 255.,
            };
            let text_brush_properties = D2D1_BRUSH_PROPERTIES {
                opacity: 1.,
                transform: Matrix3x2::identity(),
            };
            unsafe { context.Clear(&clearcolor) };
            let text_brush =
                unsafe { context.CreateSolidColorBrush(&text_color, &text_brush_properties) }?;
            unsafe {
                context.DrawTextLayout(
                    D2D_POINT_2F { x: 0., y: 0. },
                    &text_layout,
                    &text_brush,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                )
            };

            unsafe { surface_interop.EndDraw() }?;
        }
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
