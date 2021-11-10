use async_object::EventStream;
use windows::Foundation::Numerics::Vector2;

#[derive(Clone, Debug)]
pub struct SlotSize(pub Vector2);

#[derive(Clone, Debug)]
pub struct MouseLeftPressed(pub Vector2);

#[derive(Clone, Debug)]
pub struct MouseLeftPressedFocused(pub Vector2);

pub trait SendSlotEvent {
    fn send_size(&mut self, event: SlotSize) -> crate::Result<()>;
    fn send_mouse_left_pressed(&mut self, event: MouseLeftPressed) -> crate::Result<()>;
    fn send_mouse_left_pressed_focused(
        &mut self,
        event: MouseLeftPressedFocused,
    ) -> crate::Result<()>;
}

pub trait ReceiveSlotEvent {
    fn on_size(&self) -> EventStream<SlotSize>;
    fn on_mouse_left_pressed(&self) -> EventStream<MouseLeftPressed>;
    fn on_mouse_left_pressed_focused(&self) -> EventStream<MouseLeftPressedFocused>;
}
