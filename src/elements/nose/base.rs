use rpi_led_matrix::LedCanvas;
use crate::face::{RenderContext, DrawPixelFn, SharedFaceState};

/// Base trait for all nose implementations
/// Defines the common interface for nose rendering
pub trait Nose: Send + Sync {
    /// Get the name of this nose type
    fn name(&self) -> &str;

    /// Get a description of this nose type
    fn description(&self) -> &str;

    /// Update nose state (if animated)
    fn update(&mut self, shared_state: &mut SharedFaceState, dt: f64);

    /// Draw the nose to the canvas
    fn draw(&self, canvas: &mut LedCanvas, context: &RenderContext,
            shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn);

    /// Clone this nose into a Box
    fn clone_box(&self) -> Box<dyn Nose>;
}

/// Nose position configuration
#[derive(Debug, Clone, Copy)]
pub struct NosePosition {
    pub center_x: f64,
    pub center_y: f64,
}

impl Default for NosePosition {
    fn default() -> Self {
        Self {
            center_x: 56.0,  // Default nose position from Arduino code
            center_y: 27.0,
        }
    }
}
