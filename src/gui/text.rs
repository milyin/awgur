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
