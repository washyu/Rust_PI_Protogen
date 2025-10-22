use rpi_led_matrix::LedCanvas;
use crate::face::{RenderContext, DrawPixelFn, SharedFaceState};

/// Base trait for all mouth implementations
/// Defines the common interface for mouth rendering and animation
pub trait Mouth: Send + Sync {
    /// Get the name of this mouth type
    fn name(&self) -> &str;

    /// Get a description of this mouth type
    fn description(&self) -> &str;

    /// Update mouth state (opening, breathing, etc.)
    fn update(&mut self, shared_state: &mut SharedFaceState, dt: f64);

    /// Draw the mouth to the canvas
    fn draw(&self, canvas: &mut LedCanvas, context: &RenderContext,
            shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn);

    /// Clone this mouth into a Box
    fn clone_box(&self) -> Box<dyn Mouth>;
}

/// Mouth animation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouthMode {
    /// Driven by audio input from microphone
    Audio,
    /// Idle breathing animation
    Breathing,
    /// Manual control via gamepad
    Manual,
}
