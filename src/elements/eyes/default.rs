use std::time::Instant;
use rpi_led_matrix::LedCanvas;
use super::base::{Eye, BlinkConfig};
use crate::face::{RenderContext, DrawPixelFn, SharedFaceState};
use crate::{PANEL_WIDTH, PANEL_HEIGHT};

/// Default blinking eyes - original Arduino protogen eyes
#[derive(Clone)]
pub struct DefaultEyes {
    blink_sec: i32,
    blink_frame: i32,
    blink_flag: bool,
    last_second: u64,
    start_time: Instant,
    config: BlinkConfig,
}

impl DefaultEyes {
    pub fn new() -> Self {
        Self {
            blink_sec: 0,
            blink_frame: 0,
            blink_flag: true,
            last_second: 0,
            start_time: Instant::now(),
            config: BlinkConfig::default(),
        }
    }

    pub fn with_config(config: BlinkConfig) -> Self {
        Self {
            blink_sec: 0,
            blink_frame: 0,
            blink_flag: true,
            last_second: 0,
            start_time: Instant::now(),
            config,
        }
    }
}

impl Eye for DefaultEyes {
    fn name(&self) -> &str {
        "Default Eyes"
    }

    fn description(&self) -> &str {
        "Original protogen eyes with blinking animation"
    }

    fn update(&mut self, shared_state: &mut SharedFaceState, _dt: f64) {
        // Update second counter
        let current_second = self.start_time.elapsed().as_secs();
        if current_second != self.last_second {
            self.blink_sec += 1;
            self.last_second = current_second;
        }

        // Blinking logic (Arduino code)
        if !shared_state.blink_enabled {
            shared_state.eye_top = 9.0;
            shared_state.eye_bottom = 1.45;
            return;
        }

        // Early return if not time to blink yet
        if self.blink_sec < self.config.interval_secs {
            shared_state.eye_top = 9.0;
            shared_state.eye_bottom = 1.45;
            return;
        }

        // Set eye positions based on CURRENT frame (before advancing)
        // This matches Arduino: check frame, set values, then advance
        if self.blink_frame == 0 {
            shared_state.eye_bottom = 2.0;
            shared_state.eye_top = 8.0;
        } else if self.blink_frame == 1 {
            shared_state.eye_bottom = 3.0;
            shared_state.eye_top = 7.0;
        } else if self.blink_frame == 2 {
            shared_state.eye_bottom = 4.0;
            shared_state.eye_top = 6.0;
        } else if self.blink_frame == 3 {
            shared_state.eye_bottom = 5.0;
            shared_state.eye_top = 5.0;
        } else if self.blink_frame == 4 {
            shared_state.eye_bottom = 6.0;
            shared_state.eye_top = 4.0;
        } else if self.blink_frame == 5 {
            shared_state.eye_bottom = 7.0;
            shared_state.eye_top = 0.1;
            self.blink_flag = false;
        }

        // Advance frame (Arduino code pattern)
        if self.blink_flag {
            self.blink_frame += 1;
        } else {
            self.blink_frame -= 1;
        }

        if self.blink_frame == -1 {
            self.blink_sec = 0;
            self.blink_frame = 0;
            self.blink_flag = true;
        }
    }

    fn draw(&self, canvas: &mut LedCanvas, context: &RenderContext,
            shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        let bright = 255.0;
        let offset_x = context.offset_x;
        let offset_y = context.offset_y;

        // Eye coordinates (Arduino original)
        let cord_y_a_x = 0.0 + offset_x;
        let cord_y_a_y = 25.0 + offset_y;
        let cord_y_b_x = 2.0 + offset_x;
        let cord_y_b_y = 31.0 + offset_y;
        let cord_y_c_x = 10.0 + offset_x;
        let cord_y_c_y = 0.0 + offset_y;
        let cord_y_d_x = 18.0 + offset_x;
        let cord_y_d_y = 24.0 + offset_y;

        let angle_y_a = shared_state.eye_bottom;
        let angle_y_b = shared_state.eye_top;
        let angle_y_c = -0.6;

        let color_zero = context.time_counter;

        // Render eyes (Arduino rendering logic)
        for x in 1..=PANEL_WIDTH {
            let mut color = color_zero + (x as f64) * 5.0;

            let y_a = (cord_y_a_x - x as f64) / angle_y_a + cord_y_a_y;
            let y_b = (cord_y_b_x - x as f64) / angle_y_b + cord_y_b_y;
            let y_c = (cord_y_c_x - x as f64) / angle_y_c + cord_y_c_y;
            let y_d = 0.8 * (x as f64 - cord_y_d_x).powi(2) + cord_y_d_y;

            for y in 0..=PANEL_HEIGHT {
                color += 5.0;
                let y_f = y as f64;

                if y_a < y_f && y_b > y_f && y_c < y_f && y_d > y_f {
                    let brightness = if y_a < y_f - 1.0 && y_b > y_f + 1.0 &&
                                        y_c < y_f - 1.0 && y_d > y_f + 1.0 {
                        bright
                    } else if y_a > y_f - 1.0 {
                        bright * (y_f - y_a).max(0.0)
                    } else if y_b < y_f + 1.0 {
                        bright * (y_b - y_f).max(0.0)
                    } else if y_c > y_f - 1.0 {
                        bright * (y_f - y_c).max(0.0)
                    } else if y_d < y_f + 1.0 {
                        bright * (y_d - y_f).max(0.0)
                    } else {
                        bright
                    };
                    draw_pixel_fn.draw(canvas, brightness, color, x, y,
                                      context.brightness, context.palette);
                }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn Eye> {
        Box::new(self.clone())
    }
}
