use rpi_led_matrix::LedCanvas;
use super::base::{Nose, NosePosition};
use crate::face::{RenderContext, DrawPixelFn, SharedFaceState};
use crate::{PANEL_WIDTH, PANEL_HEIGHT};

/// Default protogen nose - simple parabolic curves
#[derive(Clone, Copy)]
pub struct DefaultNose {
    position: NosePosition,
}

impl DefaultNose {
    pub fn new() -> Self {
        Self {
            position: NosePosition::default(),
        }
    }

    pub fn with_position(position: NosePosition) -> Self {
        Self { position }
    }
}

impl Nose for DefaultNose {
    fn name(&self) -> &str {
        "Default Nose"
    }

    fn description(&self) -> &str {
        "Original protogen nose with parabolic curves"
    }

    fn update(&mut self, _shared_state: &mut SharedFaceState, _dt: f64) {
        // Nose is static, no update needed
    }

    fn draw(&self, canvas: &mut LedCanvas, context: &RenderContext,
            _shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        let bright = 255.0;
        let offset_x = context.offset_x;
        let offset_y = context.offset_y;

        // Nose coordinates (Arduino original)
        let cord_n_a_x = self.position.center_x + offset_x;
        let cord_n_a_y = self.position.center_y + offset_y;
        let cord_n_b_x = 53.0 + offset_x;
        let cord_n_b_y = 23.0 + offset_y;

        let color_zero = context.time_counter;

        // Render nose
        for x in 1..=PANEL_WIDTH {
            let mut color = color_zero + (x as f64) * 5.0;

            let n_a = -0.5 * (x as f64 - cord_n_a_x).powi(2) + cord_n_a_y;
            let n_b = -0.1 * (x as f64 - cord_n_b_x).powi(2) + cord_n_b_y;

            for y in 0..=PANEL_HEIGHT {
                color += 5.0;
                let y_f = y as f64;

                if n_b < y_f && n_a > y_f {
                    draw_pixel_fn.draw(canvas, bright, color, x, y,
                                      context.brightness, context.palette);
                }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn Nose> {
        Box::new(*self)
    }
}
