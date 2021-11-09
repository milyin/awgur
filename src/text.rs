use async_object::{Keeper, Tag};
use bindings::Windows::UI::Composition::{CompositionDrawingSurface, SpriteVisual};

use crate::{FrameTag, SlotTag};

struct Text {
    refs: TextRefs,
    text: String,
}

#[derive(Clone)]
struct TextRefs {
    frame: FrameTag,
    slot: SlotTag,
    surface: Option<CompositionDrawingSurface>,
    visual: SpriteVisual,
}

impl TextRefs {
    fn new(frame: FrameTag, slot: SlotTag) -> crate::Result<Self> {
        let shape = frame.compositor().CreateShapeVisual()?;
        slot.container().Children()?.InsertAtTop(shape.clone())?;
        shape.SetSize(slot.container().Size()?)?;
        Ok(Self { frame, slot, shape })
    }
}

#[derive(Clone)]
struct TextKeeper {
    keeper: Keeper<Text>,
}

#[derive(Clone)]
struct TextTag {
    tag: Tag<Text>,
    refs: TextRefs,
}

impl PartialEq for TextTag {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
    }
}
