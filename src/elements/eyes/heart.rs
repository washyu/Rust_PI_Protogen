use std::time::Instant;
use rpi_led_matrix::LedCanvas;
use super::base::{Eye, EyePosition};
use crate::face::{RenderContext, DrawPixelFn, SharedFaceState};
use crate::{PANEL_WIDTH, PANEL_HEIGHT};

/// Heart-shaped eyes - cute expression
#[derive(Clone)]
pub struct HeartEyes {
    position: EyePosition,
    blink_sec: i32,
    blink_frame: i32,
    blink_flag: bool,
    last_second: u64,
    start_time: Instant,
}

const HEART_WIDTH: i32 = 24;
const HEART_HEIGHT: i32 = 16;

// Heart bitmap pattern (24x16)
const HEART_PATTERN: [[u8; 24]; 16] = [
    [0,0,0,1,1,1,1,1,1,0,0,0,0,0,0,1,1,1,1,1,1,0,0,0],
    [0,0,1,1,1,1,1,1,1,1,0,0,0,0,1,1,1,1,1,1,1,1,0,0],
    [0,1,1,1,1,1,1,1,1,1,1,1,0,1,1,1,1,1,1,1,1,1,1,0],
    [0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0],
    [0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0],
    [0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0],
    [0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0],
    [0,0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0],
    [0,0,0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0],
    [0,0,0,0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0],
    [0,0,0,0,0,0,0,1,1,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0],
    [0,0,0,0,0,0,0,0,1,1,1,1,1,1,1,1,0,0,0,0,0,0,0,0],
    [0,0,0,0,0,0,0,0,0,1,1,1,1,1,1,0,0,0,0,0,0,0,0,0],
    [0,0,0,0,0,0,0,0,0,0,1,1,1,1,0,0,0,0,0,0,0,0,0,0],
    [0,0,0,0,0,0,0,0,0,0,0,1,1,0,0,0,0,0,0,0,0,0,0,0],
    [0,0,0,0,0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0,0,0,0,0],
];


impl HeartEyes {
    pub fn new() -> Self {
        Self {
            position: EyePosition::default(),
            blink_sec: 0,
            blink_frame: 0,
            blink_flag: true,
            last_second: 0,
            start_time: Instant::now(),
        }
    }

    pub fn with_position(position: EyePosition) -> Self {
        Self {
            position,
            blink_sec: 0,
            blink_frame: 0,
            blink_flag: true,
            last_second: 0,
            start_time: Instant::now(),
        }
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
        // Update second counter
        let current_second = self.start_time.elapsed().as_secs();
        if current_second != self.last_second {
            self.blink_sec += 1;
            self.last_second = current_second;
        }

        // Only blink if enabled
        if !shared_state.blink_enabled {
            self.blink_frame = 0;
            self.blink_sec = 0;
            return;
        }

        // Start blink after 10 seconds
        if self.blink_sec < 10 {
            return;
        }

        // Advance blink animation
        if self.blink_flag {
            self.blink_frame += 1;
            if self.blink_frame > 7 {  // Close fully at frame 7 (8 rows from top/bottom)
                self.blink_flag = false;
            }
        } else {
            self.blink_frame -= 1;
            if self.blink_frame < 0 {
                self.blink_sec = 0;
                self.blink_frame = 0;
                self.blink_flag = true;
            }
        }
    }

    fn draw(&self, canvas: &mut LedCanvas, context: &RenderContext,
            _shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        let bright = 255.0;
        let offset_x = context.offset_x;
        let offset_y = context.offset_y;

        // Calculate top-left corner to center the heart at the eye position
        let start_x = (self.position.center_x + offset_x - (HEART_WIDTH as f64 / 2.0)) as i32;
        let start_y = (self.position.center_y + offset_y - (HEART_HEIGHT as f64 / 2.0)) as i32;

        // Draw heart using bitmap pattern (flip vertically for correct orientation)
        // Apply blink effect by masking rows from top and bottom towards middle
        for row in 0..HEART_HEIGHT {
            for col in 0..HEART_WIDTH {
                let flipped_row = (HEART_HEIGHT - 1 - row) as usize;

                // Check if this row should be masked during blink
                let should_draw = if self.blink_frame > 0 {
                    // Mask from top and bottom towards middle
                    let rows_from_top = self.blink_frame;
                    let rows_from_bottom = self.blink_frame;
                    row >= rows_from_top && row < (HEART_HEIGHT - rows_from_bottom)
                } else {
                    true
                };

                if should_draw && HEART_PATTERN[flipped_row][col as usize] == 1 {
                    let x = start_x + col;
                    let y = start_y + row;

                    // Check bounds
                    if x >= 1 && x <= PANEL_WIDTH && y >= 0 && y <= PANEL_HEIGHT {
                        // Calculate color with shimmer effect
                        let color = context.time_counter + (x as f64) * 5.0 + (y as f64) * 5.0;
                        draw_pixel_fn.draw(canvas, bright, color, x, y,
                                          context.brightness, context.palette);
                    }
                }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn Eye> {
        Box::new(self.clone())
    }
}
