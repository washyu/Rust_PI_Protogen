use rpi_led_matrix::LedCanvas;
use super::base::{Eye, EyePosition};
use crate::face::{RenderContext, DrawPixelFn, SharedFaceState};
use crate::{PANEL_WIDTH, PANEL_HEIGHT};

/// Circle eyes - surprised/shocked expression
#[derive(Clone)]
pub struct CircleEyes {
    position: EyePosition,
}

impl CircleEyes {
    pub fn new() -> Self {
        Self {
            position: EyePosition::default(),
        }
    }

    pub fn with_position(position: EyePosition) -> Self {
        Self { position }
    }
}

impl Eye for CircleEyes {
    fn name(&self) -> &str {
        "O Eyes"
    }

    fn description(&self) -> &str {
        "Circle eyes - surprised expression"
    }

    fn update(&mut self, shared_state: &mut SharedFaceState, _dt: f64) {
        // Circles don't blink
        shared_state.eye_top = 9.0;
        shared_state.eye_bottom = 1.45;
    }

    fn draw(&self, canvas: &mut LedCanvas, context: &RenderContext,
            _shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        let bright = 255.0;
        let offset_x = context.offset_x;
        let offset_y = context.offset_y;

        // Draw one circle positioned at the eye location (will be mirrored by draw_pixel_fn)
        let cx = self.position.center_x + offset_x;
        let cy = self.position.center_y + offset_y;

        for x in 1..=PANEL_WIDTH {
            let mut color = context.time_counter + (x as f64) * 5.0;

            for y in 0..=PANEL_HEIGHT {
                color += 5.0;
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                let dist_sq = dx * dx + dy * dy;

                // Hollow circle (ring)
                if dist_sq > 16.0 && dist_sq < 36.0 {
                    draw_pixel_fn.draw(canvas, bright, color, x, y,
                                      context.brightness, context.palette);
                }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn Eye> {
        Box::new(self.clone())
    }
}
