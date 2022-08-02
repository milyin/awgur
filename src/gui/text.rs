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
        Foundation::HINSTANCE,
        Graphics::{
            Direct2D::{
                D2D1CreateFactory, ID2D1Device, ID2D1Factory1, D2D1_FACTORY_OPTIONS,
                D2D1_FACTORY_TYPE_SINGLE_THREADED,
            },
            Direct3D::{D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                D3D11_SDK_VERSION,
            },
            DirectWrite::{
                DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED,
                DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BOLD,
                DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER,
            },
            Dxgi::IDXGIDevice,
        },
        System::WinRT::Composition::{ICompositionDrawingSurfaceInterop, ICompositorInterop},
    },
    UI::Composition::{
        CompositionGraphicsDevice, CompositionStretch, CompositionSurfaceBrush, Compositor,
        ICompositionSurface, SpriteVisual, Visual,
    },
};

use super::{EventSink, EventSource, Panel, PanelEvent};

thread_local! {
    static DWRITE_FACTORY: Result<IDWriteFactory, windows::core::Error> = create_dwrite_factory();
    static D3D11_DEVICE: Result<ID3D11Device, windows::core::Error> = create_d3d11_device();
    static D2D1_DEVICE: Result<windows::Win32::Graphics::Direct2D::ID2D1Device, windows::core::Error> = create_d2d1_device();
}

fn create_dwrite_factory() -> Result<IDWriteFactory, windows::core::Error> {
    let dwrite_factory =
        unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, &IDWriteFactory::IID) }?;
    Ok(dwrite_factory.cast()?)
}

fn dwrite_factory() -> crate::Result<IDWriteFactory> {
    DWRITE_FACTORY.with(|v| match v {
        Ok(v) => Ok(v.clone()),
        Err(e) => Err(crate::Error::Windows(e.clone())),
    })
}

fn create_d3d11_device() -> Result<ID3D11Device, windows::core::Error> {
    fn create_device(driver_type: D3D_DRIVER_TYPE) -> Result<ID3D11Device, windows::core::Error> {
        let mut device: Option<ID3D11Device> = None;
        unsafe {
            D3D11CreateDevice(
                InParam::null(),
                driver_type,
                HINSTANCE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                &[],
                D3D11_SDK_VERSION,
                &mut device,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        }?;
        Ok(device.unwrap())
    }

    let device = create_device(D3D_DRIVER_TYPE_HARDWARE);
    let device = if device.is_ok() {
        device
    } else {
        create_device(D3D_DRIVER_TYPE_WARP)
    };
    device
}

fn d3d11_device() -> crate::Result<ID3D11Device> {
    D3D11_DEVICE.with(|v| match v {
        Ok(v) => Ok(v.clone()),
        Err(e) => Err(crate::Error::Windows(e.clone())),
    })
}

fn create_d2d1_device() -> Result<ID2D1Device, windows::core::Error> {
    let dxdevice: IDXGIDevice = D3D11_DEVICE.with(|v| v.clone())?.cast()?;
    let options = D2D1_FACTORY_OPTIONS::default();
    let factory: ID2D1Factory1 =
        unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, &options) }?;
    let d2device = unsafe { factory.CreateDevice(&dxdevice) }?;
    Ok(d2device)
}

fn d2d1_device() -> crate::Result<ID2D1Device> {
    D2D1_DEVICE.with(|v| match v {
        Ok(v) => Ok(v.clone()),
        Err(e) => Err(crate::Error::Windows(e.clone())),
    })
}

fn composition_graphics_device(
    compositor: &Compositor,
) -> crate::Result<CompositionGraphicsDevice> {
    let interop_compositor: ICompositorInterop = compositor.cast()?;
    let d2device = d2d1_device()?;
    let graphic_device = unsafe { interop_compositor.CreateGraphicsDevice(&d2device) }?;
    Ok(graphic_device)
}

struct Core {
    compositor: Compositor,
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
        let surface_brush = compositor.CreateSurfaceBrush()?;
        surface_brush.SetStretch(CompositionStretch::None)?;
        surface_brush.SetHorizontalAlignmentRatio(0.)?;
        surface_brush.SetVerticalAlignmentRatio(0.)?;
        // surface_brush.SetTransformMatrix(Matrix3x2::translation(20., 20.))?;
        sprite_visual.SetBrush(&surface_brush)?;
        Ok(Self {
            compositor,
            text,
            surface_brush,
            sprite_visual,
        })
    }
    fn resize(&mut self, size: Vector2) -> crate::Result<()> {
        self.init(&size)?;
        self.sprite_visual.SetSize(size)?;
        Ok(())
    }
    fn init(&mut self, size: &Vector2) -> crate::Result<()> {
        let graphic_device = composition_graphics_device(&self.compositor)?;
        let virtual_surface = graphic_device.CreateVirtualDrawingSurface(
            SizeInt32 {
                Width: size.X as i32,
                Height: size.Y as i32,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )?;

        let surface_interop: ICompositionDrawingSurfaceInterop = virtual_surface.cast()?;

        let surface: ICompositionSurface = surface_interop.cast()?;

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
