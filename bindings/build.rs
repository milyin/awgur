fn main() {
    windows::core::build! {
        Microsoft::Graphics::Canvas::CanvasDevice,
        Microsoft::Graphics::Canvas::UI::Composition::CanvasComposition,
        Microsoft::Graphics::Canvas::Text::CanvasHorizontalAlignment,
        Microsoft::Graphics::Canvas::Text::CanvasTextFormat,
        Microsoft::Graphics::Canvas::Text::CanvasTextLayout,
    };
}
