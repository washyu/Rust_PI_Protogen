use rpi_led_matrix::LedCanvas;
use super::base::{Eye, EyePosition};
use crate::face::{RenderContext, DrawPixelFn, SharedFaceState};
use crate::{PANEL_WIDTH, PANEL_HEIGHT};

/// Heart-shaped eyes - cute expression
#[derive(Clone)]
pub struct HeartEyes {
    position: EyePosition,
}

impl HeartEyes {
    pub fn new() -> Self {
        Self {
            position: EyePosition::default(),
        }
    }

    pub fn with_position(position: EyePosition) -> Self {
        Self { position }
    }
}

impl Eye for HeartEyes {
    fn name(&self) -> &str {
        "Heart Eyes"
    }

    fn description(&self) -> &str {
        "Heart-shaped eyes - cute expression"
    }

    fn update(&mut self, shared_state: &mut SharedFaceState, _dt: f64) {
        // Hearts don't blink
        shared_state.eye_top = 9.0;
        shared_state.eye_bottom = 1.45;
    }

    fn draw(&self, canvas: &mut LedCanvas, context: &RenderContext,
            _shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        let bright = 255.0;
        let offset_x = context.offset_x;
        let offset_y = context.offset_y;

        // Draw one heart positioned at the eye location (will be mirrored by draw_pixel_fn)
        let cx = self.position.center_x + offset_x;
        let cy = self.position.center_y + offset_y;

        for x in 1..=PANEL_WIDTH {
            let mut color = context.time_counter + (x as f64) * 5.0;

            for y in 0..=PANEL_HEIGHT {
                color += 5.0;
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;

                // Heart shape using implicit function
                let heart = (dx * dx + dy * dy - 25.0).powi(3) -
                            dx * dx * dy * dy * dy;

                if heart < 100.0 && dy > -5.0 {
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
