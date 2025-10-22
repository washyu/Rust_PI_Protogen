use rpi_led_matrix::LedCanvas;
use crate::face::{RenderContext, DrawPixelFn, SharedFaceState};

/// Base trait for all eye implementations
/// Defines the common interface for eye rendering and animation
pub trait Eye: Send + Sync {
    /// Get the name of this eye type
    fn name(&self) -> &str;

    /// Get a description of this eye type
    fn description(&self) -> &str;

    /// Update eye state (blinking, animation, etc.)
    fn update(&mut self, shared_state: &mut SharedFaceState, dt: f64);

    /// Draw the eye to the canvas
    fn draw(&self, canvas: &mut LedCanvas, context: &RenderContext,
            shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn);

    /// Clone this eye into a Box
    fn clone_box(&self) -> Box<dyn Eye>;
}

/// Eye position configuration
#[derive(Debug, Clone, Copy)]
pub struct EyePosition {
    pub center_x: f64,
    pub center_y: f64,
}

impl Default for EyePosition {
    fn default() -> Self {
        Self {
            center_x: 13.0,  // Default eye position from Arduino code
            center_y: 22.0,
        }
    }
}

/// Blink animation configuration
#[derive(Debug, Clone, Copy)]
pub struct BlinkConfig {
    pub enabled: bool,
    pub interval_secs: i32,  // Seconds between blinks
    pub frames: i32,         // Number of frames in blink animation
}

impl Default for BlinkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 10,
            frames: 6,
        }
    }
}
