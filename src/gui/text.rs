use std::{borrow::Cow, sync::Arc};

use async_event_streams::{
    spawn_event_pipe, EventBox, EventSink, EventSinkExt, EventSource, EventStream, EventStreams,
};
use async_event_streams_derive::EventSink;
use async_std::sync::RwLock;
use async_trait::async_trait;
use futures::task::Spawn;
use typed_builder::TypedBuilder;
use windows::{
    core::InParam,
    w,
    Foundation::Numerics::{Matrix3x2, Vector2},
    Graphics::SizeInt32,
    Win32::Graphics::{
        Direct2D::{
            Common::{D2D1_COLOR_F, D2D_RECT_F},
            D2D1_BRUSH_PROPERTIES, D2D1_DRAW_TEXT_OPTIONS_NONE,
        },
        DirectWrite::{
            DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_ITALIC, DWRITE_FONT_WEIGHT_BOLD,
            DWRITE_MEASURING_MODE_NATURAL,
        },
    },
    UI::Composition::{CompositionDrawingSurface, Compositor, Visual},
};

use crate::window::{draw, dwrite_factory, ToWide};

use super::{surface::SurfaceEvent, Panel, PanelEvent, Surface, SurfaceParams};

#[derive(EventSink)]
#[event_sink(event=SurfaceEvent)]
struct Core {
    surface: Arc<Surface>,
    text: String,
}

impl Core {
    fn new(surface: Arc<Surface>, text: String) -> crate::Result<Self> {
        Ok(Self { surface, text })
    }
}

fn redraw(size: Vector2, surface: &CompositionDrawingSurface, text: &str) -> crate::Result<()> {
    let new_surface_size = SizeInt32 {
        Width: size.X as i32,
        Height: size.Y as i32,
    };
    surface.Resize(new_surface_size)?;
    draw(surface, |context, point| {
        let fontsize = 30.;
        let dwrite_text_format = unsafe {
            dwrite_factory()?.CreateTextFormat(
                w!("Segoe UI"),
                InParam::null(),
                DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_STYLE_ITALIC,
                DWRITE_FONT_STRETCH_NORMAL,
                fontsize,
                w!("en-US"),
            )
        }?;

        let clearcolor = D2D1_COLOR_F {
            r: 0.,
            g: 30.,
            b: 30.,
            a: 255.,
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
            context.DrawText(
                text.to_wide().0.as_slice(),
                &dwrite_text_format,
                &D2D_RECT_F {
                    left: point.x as f32,
                    top: point.y as f32,
                    right: point.x as f32 + size.X,
                    bottom: point.y as f32 + size.Y,
                },
                &text_brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );
            /*
            context.DrawTextLayout(
                D2D_POINT_2F {
                    x: 0.,
                    y: size.Y / 2.,
                },
                &text_layout,
                &text_brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            )
            */
        };

        Ok(())
    })?;
    Ok(())
}

#[async_trait]
impl EventSinkExt<SurfaceEvent> for Core {
    type Error = crate::Error;
    async fn on_event<'a>(
        &'a self,
        event: Cow<'a, SurfaceEvent>,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        match event.as_ref() {
            SurfaceEvent::Redraw(size) => {
                redraw(*size, self.surface.surface(), self.text.as_str())?
            }
        }
        Ok(())
    }
}

#[derive(EventSink)]
#[event_sink(event=PanelEvent)]
pub struct Text {
    surface: Arc<Surface>,
    core: Arc<RwLock<Core>>,
    panel_events: EventStreams<PanelEvent>,
    id: Arc<()>,
}

/*
impl Text {
    fn resize(&mut self, size: Vector2) -> crate::Result<()> {
        self.sprite_visual.SetSize(size)?;
        let new_surface_size = SizeInt32 {
            Width: size.X as i32,
            Height: size.Y as i32,
        };
        self.surface.Resize(new_surface_size)?;
        self.redraw(&size)?;
        Ok(())
    }
    fn redraw(&mut self, size: &Vector2) -> crate::Result<()> {
        let fontsize = size.Y;
        // let fontsize = 30.;
        let dwrite_text_format = unsafe {
            dwrite_factory()?.CreateTextFormat(
                w!("Segoe UI"),
                InParam::null(),
                DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_FONT_STYLE_ITALIC,
                DWRITE_FONT_STRETCH_NORMAL,
                fontsize,
                w!("en-US"),
            )
        }?;
        unsafe { dwrite_text_format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER) }?;
        unsafe { dwrite_text_format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER) }?;
        // unsafe { dwrite_text_format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING) }?;
        let text_layout = unsafe {
            dwrite_factory()?.CreateTextLayout(
                self.text.as_str().to_wide().0.as_slice(),
                &dwrite_text_format,
                size.X,
                size.Y / 2.,
            )
        }?;

        let mut updateoffset = POINT { x: 0, y: 0 };
        let surface_interop: ICompositionDrawingSurfaceInterop = self.surface.cast()?;
        let context: Option<ID2D1DeviceContext> = check_for_device_removed(unsafe {
            surface_interop.BeginDraw(std::ptr::null(), &mut updateoffset)
        })?;
        if let Some(context) = context {
            let clearcolor = D2D1_COLOR_F {
                r: 0.,
                g: 30.,
                b: 30.,
                a: 255.,
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
                context.DrawText(
                    self.text.as_str().to_wide().0.as_slice(),
                    &dwrite_text_format,
                    &D2D_RECT_F {
                        left: updateoffset.x as f32,
                        top: updateoffset.y as f32,
                        right: updateoffset.x as f32 + size.X,
                        bottom: updateoffset.y as f32 + size.Y,
                    },
                    &text_brush,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
                /*
                context.DrawTextLayout(
                    D2D_POINT_2F {
                        x: 0.,
                        y: size.Y / 2.,
                    },
                    &text_layout,
                    &text_brush,
                    D2D1_DRAW_TEXT_OPTIONS_NONE,
                )
                */
            };
            unsafe { surface_interop.EndDraw() }?;
        }
        Ok(())
    }
}

*/

#[async_trait]
impl EventSinkExt<PanelEvent> for Text {
    type Error = crate::Error;
    async fn on_event<'a>(
        &'a self,
        event: Cow<'a, PanelEvent>,
        source: Option<Arc<EventBox>>,
    ) -> crate::Result<()> {
        self.surface
            .on_event_ref(event.as_ref(), source.clone())
            .await?;
        self.panel_events
            .send_event(event.into_owned(), source)
            .await;
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
        self.surface.outer_frame()
    }
    fn id(&self) -> usize {
        Arc::as_ptr(&self.id) as usize
    }
}

#[derive(TypedBuilder)]
pub struct TextParams<T: Spawn> {
    compositor: Compositor,
    text: String,
    spawner: T,
}

impl<T: Spawn> TryFrom<TextParams<T>> for Text {
    type Error = crate::Error;

    fn try_from(value: TextParams<T>) -> crate::Result<Self> {
        let surface: Arc<Surface> = SurfaceParams::builder()
            .compositor(value.compositor)
            .build()
            .try_into()?;
        let core = Arc::new(RwLock::new(Core::new(surface.clone(), value.text)?));

        spawn_event_pipe(&value.spawner, &surface, core.clone(), |e| panic!());
        Ok(Text {
            surface,
            core,
            panel_events: EventStreams::new(),
            id: Arc::new(()),
        })
    }
}

impl<T: Spawn> TryFrom<TextParams<T>> for Arc<Text> {
    type Error = crate::Error;

    fn try_from(value: TextParams<T>) -> crate::Result<Self> {
        Ok(Arc::new(value.try_into()?))
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
