// Module declarations
mod audio;
mod color;
mod gamepad;

use rpi_led_matrix::{LedMatrix, LedMatrixOptions, LedCanvas};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::any::Any;
use gilrs::{Gilrs, Button};

// Re-export from modules
use audio::{AudioLevel, start_audio_capture, SILENT_LIMIT};
use color::{ColorPalette, get_shimmer_color};
use gamepad::{MaskState, handle_gamepad_input, CycleEyes};

const PANEL_WIDTH: i32 = 64;
const PANEL_HEIGHT: i32 = 32;

// Microphone constants (matching Arduino code)
const MOUTH_MAX_OPENING: f64 = 6.0;
const IDLE_TIMEOUT_SECS: u64 = 30; // Switch to breathing after 30 seconds of silence

// ============================================================================
// FACE ELEMENT REGISTRY SYSTEM
// ============================================================================
// Allows modular, swappable face components (eyes, mouth, nose, accessories)
// Each element handles its own rendering, state, and input

// Element categories for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ElementCategory {
    Eyes,
    Mouth,
    Nose,
    Accessory, // Blush, tears, etc.
}

// Context passed to elements during rendering
struct RenderContext {
    // Canvas for drawing
    // Position offsets from head movement
    offset_x: f64,
    offset_y: f64,
    // Animation time
    time_counter: f64,
    // Current brightness and palette
    brightness: f64,
    palette: ColorPalette,
}

// Shared state that elements can read/write
struct SharedFaceState {
    mouth_opening: f64,  // 0.0 to MOUTH_MAX_OPENING
    eye_top: f64,        // Top eyelid position
    eye_bottom: f64,     // Bottom eyelid position
    blink_enabled: bool,
}

// Trait for all face elements
trait FaceElement {
    // Metadata
    fn name(&self) -> &str;
    fn category(&self) -> ElementCategory;
    fn description(&self) -> &str { "" }

    // Lifecycle - called every frame
    fn update(&mut self, shared_state: &mut SharedFaceState, dt: f64);

    // Rendering - draw to canvas
    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
              shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn);

    // Input handling - return true if button was handled
    fn handle_button(&mut self, _button: Button, _shared_state: &mut SharedFaceState) -> bool {
        false
    }

    // Debug info for status display
    fn status(&self) -> String { String::new() }

    // Downcasting support for accessing element-specific methods
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// Helper trait for drawing pixels with state
trait DrawPixelFn {
    fn draw(&self, canvas: &mut LedCanvas, bright: f64, color_index: f64,
            x: i32, y: i32, brightness: f64, palette: ColorPalette);
}


// Pixel drawer implementation
struct PixelDrawer;

impl DrawPixelFn for PixelDrawer {
    fn draw(&self, canvas: &mut LedCanvas, bright_f: f64, color_index: f64,
            x: i32, y: i32, brightness: f64, palette: ColorPalette) {
        // Rotate 180 degrees: new_x = width - x, new_y = height - y
        let rotated_x = PANEL_WIDTH - x;
        let rotated_y = PANEL_HEIGHT - 1 - y;

        if rotated_x < 0 || rotated_x >= PANEL_WIDTH || rotated_y < 0 || rotated_y >= PANEL_HEIGHT {
            return;
        }

        let adjusted_brightness = bright_f * brightness;
        let color = get_shimmer_color(color_index, adjusted_brightness, palette);

        // Draw on left panel
        canvas.set(rotated_x, rotated_y, &color);

        // Mirror on right panel
        let mirror_x = (PANEL_WIDTH * 2) - 1 - rotated_x;
        if mirror_x >= PANEL_WIDTH && mirror_x < PANEL_WIDTH * 2 {
            canvas.set(mirror_x, rotated_y, &color);
        }
    }
}

// Face element registry - manages all active face elements
struct FaceElementRegistry {
    elements: Vec<Box<dyn FaceElement>>,
    active_eyes_index: usize,
    eyes_variants: Vec<String>, // Names of available eye variants
}

impl FaceElementRegistry {
    fn new() -> Self {
        Self {
            elements: Vec::new(),
            active_eyes_index: 0,
            eyes_variants: Vec::new(),
        }
    }

    fn register(&mut self, element: Box<dyn FaceElement>) {
        // Track eye variants for cycling
        if element.category() == ElementCategory::Eyes {
            self.eyes_variants.push(element.name().to_string());
        }
        self.elements.push(element);
    }

    fn update_all(&mut self, shared_state: &mut SharedFaceState, dt: f64) {
        for element in &mut self.elements {
            element.update(shared_state, dt);
        }
    }

    fn render_all(&self, canvas: &mut LedCanvas, context: &RenderContext,
                  shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        // Render in order: mouth, nose, eyes, accessories
        let order = [ElementCategory::Mouth, ElementCategory::Nose,
                     ElementCategory::Eyes, ElementCategory::Accessory];

        for category in &order {
            for (idx, element) in self.elements.iter().enumerate() {
                if element.category() != *category {
                    continue;
                }
                // Skip non-active eye variants
                if *category == ElementCategory::Eyes {
                    let eye_idx = self.eyes_variants.iter()
                        .position(|n| n == element.name());
                    if let Some(ei) = eye_idx {
                        if ei != self.active_eyes_index {
                            continue;
                        }
                    }
                }
                element.render(canvas, context, shared_state, draw_pixel_fn);
            }
        }
    }

    fn handle_button(&mut self, button: Button, shared_state: &mut SharedFaceState) -> bool {
        // Let elements handle input
        for element in &mut self.elements {
            if element.handle_button(button, shared_state) {
                return true;
            }
        }
        false
    }

    fn cycle_eyes(&mut self) {
        if !self.eyes_variants.is_empty() {
            self.active_eyes_index = (self.active_eyes_index + 1) % self.eyes_variants.len();
            println!("👁️  Eyes: {}", self.eyes_variants[self.active_eyes_index]);
        }
    }

    fn get_active_eyes_name(&self) -> String {
        self.eyes_variants.get(self.active_eyes_index)
            .cloned()
            .unwrap_or_else(|| "None".to_string())
    }
}

// Mask control state
#[derive(Debug, Clone)]

// Audio level tracker


// Color palette for shimmer effect with multiple color schemes

// ============================================================================
// DEFAULT FACE ELEMENTS
// ============================================================================

// DEFAULT EYES - Original blinking eyes from Arduino code
struct DefaultEyes {
    blink_sec: i32,
    blink_frame: i32,
    blink_flag: bool,
    last_second: u64,
    start_time: Instant,
}

impl DefaultEyes {
    fn new() -> Self {
        Self {
            blink_sec: 0,
            blink_frame: 0,
            blink_flag: true,
            last_second: 0,
            start_time: Instant::now(),
        }
    }
}

impl FaceElement for DefaultEyes {
    fn name(&self) -> &str { "Default Eyes" }
    fn category(&self) -> ElementCategory { ElementCategory::Eyes }
    fn description(&self) -> &str { "Original protogen eyes with blinking" }

    fn update(&mut self, shared_state: &mut SharedFaceState, _dt: f64) {
        // Update second counter
        let current_second = self.start_time.elapsed().as_secs();
        if current_second != self.last_second {
            self.blink_sec += 1;
            self.last_second = current_second;
        }

        // Blinking logic (Arduino code)
        if !shared_state.blink_enabled || self.blink_sec < 10 {
            shared_state.eye_top = 9.0;
            shared_state.eye_bottom = 1.45;
            return;
        }

        let (eye_bottom, eye_top) = match self.blink_frame {
            0 => (2.0, 8.0),
            1 => (3.0, 7.0),
            2 => (4.0, 6.0),
            3 => (5.0, 5.0),
            4 => (6.0, 4.0),
            5 => {
                self.blink_flag = false;
                (7.0, 0.1)
            }
            _ => (1.45, 9.0),
        };

        shared_state.eye_bottom = eye_bottom;
        shared_state.eye_top = eye_top;

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

    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
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

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// DEFAULT MOUTH - Audio-reactive mouth with microphone support
struct DefaultMouth {
    mouth_opening: f64,
    breathing_phase: f64,
    audio_level: Arc<AudioLevel>,
}

impl DefaultMouth {
    fn new(audio_level: Arc<AudioLevel>) -> Self {
        Self {
            mouth_opening: 0.0,
            breathing_phase: 0.0,
            audio_level,
        }
    }
}

impl FaceElement for DefaultMouth {
    fn name(&self) -> &str { "Default Mouth" }
    fn category(&self) -> ElementCategory { ElementCategory::Mouth }
    fn description(&self) -> &str { "Audio-reactive mouth with microphone input" }

    fn update(&mut self, shared_state: &mut SharedFaceState, dt: f64) {
        // Determine if using mic or breathing
        let seconds_idle = self.audio_level.seconds_since_audio();
        let use_breathing = seconds_idle >= IDLE_TIMEOUT_SECS;

        if use_breathing {
            // Breathing animation
            self.breathing_phase += 0.05;
            let breathing = (self.breathing_phase.sin() + 1.0) / 2.0;
            let target_mouth = breathing * MOUTH_MAX_OPENING;

            if self.mouth_opening < target_mouth {
                self.mouth_opening += 0.1;
            } else {
                self.mouth_opening -= 0.1;
            }
        } else {
            // Microphone input
            let mic_level = self.audio_level.get_level();

            if mic_level > SILENT_LIMIT {
                self.mouth_opening += 1.5;
            } else {
                self.mouth_opening -= 0.8;
            }
        }

        // Clamp
        self.mouth_opening = self.mouth_opening.clamp(0.0, MOUTH_MAX_OPENING);
        shared_state.mouth_opening = self.mouth_opening;
    }

    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
              shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        let bright = 255.0;
        let offset_x = context.offset_x;
        let offset_y = context.offset_y;
        let mouth = shared_state.mouth_opening;

        // Mouth coordinates (Arduino original)
        let cord_m_a_x = 7.0 + offset_x;
        let cord_m_a_y = 31.0 + offset_y;
        let cord_m_b_x = 7.0 + offset_x;
        let cord_m_b_y = 18.0 + offset_y + mouth / 2.0;
        let cord_m_c_x = 0.0 + offset_x;
        let cord_m_c_y = -32.0 + offset_y;
        let cord_m_d_x = 0.0 + offset_x;
        let cord_m_d_y = -37.0 + offset_y - mouth;
        let cord_m_e_x = 0.0 + offset_x;
        let cord_m_e_y = 57.0 + offset_y;
        let cord_m_f_x = 0.0 + offset_x;
        let cord_m_f_y = 52.0 + offset_y - mouth * 1.3;
        let cord_m_g_x = 0.0 + offset_x;
        let cord_m_g_y = -2.0 + offset_y;

        let angle_m_a = 1.3;
        let angle_m_b = 1.9 - mouth / 10.0;
        let angle_m_c = -1.2;
        let angle_m_d = -1.2;
        let angle_m_e = 1.2;
        let angle_m_f = 1.2;
        let angle_m_g = -1.6;

        let color_zero = context.time_counter;

        // Render mouth
        for x in 1..=PANEL_WIDTH {
            let mut color = color_zero + (x as f64) * 5.0;

            let m_a = (cord_m_a_x - x as f64) / angle_m_a + cord_m_a_y;
            let m_b = (cord_m_b_x - x as f64) / angle_m_b + cord_m_b_y;
            let m_c = (cord_m_c_x - x as f64) / angle_m_c + cord_m_c_y;
            let m_d = (cord_m_d_x - x as f64) / angle_m_d + cord_m_d_y;
            let m_e = (cord_m_e_x - x as f64) / angle_m_e + cord_m_e_y;
            let m_f = (cord_m_f_x - x as f64) / angle_m_f + cord_m_f_y;
            let m_g = (cord_m_g_x - x as f64) / angle_m_g + cord_m_g_y;

            for y in 0..=PANEL_HEIGHT {
                color += 5.0;
                let y_f = y as f64;

                if (m_e > y_f && m_f < y_f && m_c > y_f) ||
                   (m_c > y_f && m_d < y_f && m_e > y_f && m_b < y_f) ||
                   (m_b < y_f && m_a > y_f && m_g > y_f && m_d < y_f) {
                    draw_pixel_fn.draw(canvas, bright, color, x, y,
                                      context.brightness, context.palette);
                }
            }
        }
    }

    fn handle_button(&mut self, button: Button, shared_state: &mut SharedFaceState) -> bool {
        match button {
            Button::LeftTrigger | Button::LeftTrigger2 => {
                shared_state.mouth_opening = MOUTH_MAX_OPENING;
                println!("😮 Mouth: OPEN (manual)");
                true
            }
            Button::RightTrigger | Button::RightTrigger2 => {
                shared_state.mouth_opening = 0.0;
                println!("😐 Mouth: CLOSED (manual)");
                true
            }
            _ => false
        }
    }

    fn status(&self) -> String {
        format!("Mouth: {:.2}", self.mouth_opening)
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// DEFAULT NOSE - Simple nose element
struct DefaultNose;

impl FaceElement for DefaultNose {
    fn name(&self) -> &str { "Default Nose" }
    fn category(&self) -> ElementCategory { ElementCategory::Nose }
    fn description(&self) -> &str { "Original protogen nose" }

    fn update(&mut self, _shared_state: &mut SharedFaceState, _dt: f64) {
        // Nose is static, no update needed
    }

    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
              _shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        let bright = 255.0;
        let offset_x = context.offset_x;
        let offset_y = context.offset_y;

        // Nose coordinates (Arduino original)
        let cord_n_a_x = 56.0 + offset_x;
        let cord_n_a_y = 27.0 + offset_y;
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

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// ============================================================================
// ALTERNATIVE FACE ELEMENTS
// ============================================================================

// HEART EYES - Cute heart-shaped eyes
struct HeartEyes;

impl FaceElement for HeartEyes {
    fn name(&self) -> &str { "Heart Eyes" }
    fn category(&self) -> ElementCategory { ElementCategory::Eyes }
    fn description(&self) -> &str { "Heart-shaped eyes - cute expression" }

    fn update(&mut self, shared_state: &mut SharedFaceState, _dt: f64) {
        // Hearts don't blink
        shared_state.eye_top = 9.0;
        shared_state.eye_bottom = 1.45;
    }

    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
              _shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        let bright = 255.0;
        let offset_x = context.offset_x;
        let offset_y = context.offset_y;

        // Draw one heart positioned at the eye location (will be mirrored by draw_pixel_fn)
        // Using cord_y_d position from DefaultEyes, adjusted down 2 rows and back 5 columns
        let cx = 13.0 + offset_x;  // 18.0 - 5.0
        let cy = 22.0 + offset_y;  // 24.0 - 2.0 (Y decreases going down on rotated display)

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

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// X EYES - Dizzy/knocked out expression
struct XEyes;

impl FaceElement for XEyes {
    fn name(&self) -> &str { "X Eyes" }
    fn category(&self) -> ElementCategory { ElementCategory::Eyes }
    fn description(&self) -> &str { "X-shaped eyes - dizzy expression" }

    fn update(&mut self, shared_state: &mut SharedFaceState, _dt: f64) {
        shared_state.eye_top = 9.0;
        shared_state.eye_bottom = 1.45;
    }

    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
              _shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        let bright = 255.0;
        let offset_x = context.offset_x;
        let offset_y = context.offset_y;

        // Draw one X positioned at the eye location (will be mirrored by draw_pixel_fn)
        // Using cord_y_d position from DefaultEyes, adjusted down 2 rows and back 5 columns
        let cx = 13.0 + offset_x;  // 18.0 - 5.0
        let cy = 22.0 + offset_y;  // 24.0 - 2.0 (Y decreases going down on rotated display)

        for x in 1..=PANEL_WIDTH {
            let mut color = context.time_counter + (x as f64) * 5.0;

            for y in 0..=PANEL_HEIGHT {
                color += 5.0;
                let dx = (x as f64 - cx).abs();
                let dy = (y as f64 - cy).abs();

                // Draw diagonal lines forming an X
                if (dx - dy).abs() < 1.5 && dx < 6.0 && dy < 6.0 {
                    draw_pixel_fn.draw(canvas, bright, color, x, y,
                                      context.brightness, context.palette);
                }
                if (dx + dy - 12.0).abs() < 1.5 && dx < 6.0 && dy < 6.0 {
                    draw_pixel_fn.draw(canvas, bright, color, x, y,
                                      context.brightness, context.palette);
                }
            }
        }
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// O EYES - Surprised/shocked expression
struct OEyes;

impl FaceElement for OEyes {
    fn name(&self) -> &str { "O Eyes" }
    fn category(&self) -> ElementCategory { ElementCategory::Eyes }
    fn description(&self) -> &str { "Circle eyes - surprised expression" }

    fn update(&mut self, shared_state: &mut SharedFaceState, _dt: f64) {
        shared_state.eye_top = 9.0;
        shared_state.eye_bottom = 1.45;
    }

    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
              _shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        let bright = 255.0;
        let offset_x = context.offset_x;
        let offset_y = context.offset_y;

        // Draw one circle positioned at the eye location (will be mirrored by draw_pixel_fn)
        // Using cord_y_d position from DefaultEyes, adjusted down 2 rows and back 5 columns
        let cx = 13.0 + offset_x;  // 18.0 - 5.0
        let cy = 22.0 + offset_y;  // 24.0 - 2.0 (Y decreases going down on rotated display)

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

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// RUNNY NOSE - Accessory element (sick/crying)
struct RunnyNose {
    drip_offset: f64,
}

impl RunnyNose {
    fn new() -> Self {
        Self {
            drip_offset: 0.0,
        }
    }
}

impl FaceElement for RunnyNose {
    fn name(&self) -> &str { "Runny Nose" }
    fn category(&self) -> ElementCategory { ElementCategory::Accessory }
    fn description(&self) -> &str { "Dripping nose - sick expression" }

    fn update(&mut self, _shared_state: &mut SharedFaceState, _dt: f64) {
        // Animate drip downward
        self.drip_offset += 0.3;
        if self.drip_offset > 10.0 {
            self.drip_offset = 0.0;
        }
    }

    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
              _shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        let bright = 200.0;
        let offset_x = context.offset_x;
        let offset_y = context.offset_y;

        // Drip from nose downward
        let drip_x = 55.0 + offset_x;
        let drip_start_y = 20.0 + offset_y;
        let drip_end_y = drip_start_y - self.drip_offset;

        for x in 1..=PANEL_WIDTH {
            let color = context.time_counter + (x as f64) * 3.0;

            for y in 0..=PANEL_HEIGHT {
                let dx = (x as f64 - drip_x).abs();
                let y_f = y as f64;

                // Draw vertical drip
                if dx < 1.0 && y_f < drip_start_y && y_f > drip_end_y {
                    draw_pixel_fn.draw(canvas, bright, color, x, y,
                                      context.brightness, context.palette);
                }
            }
        }
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// Main face manager using the registry system
struct ProtogenFace {
    time_counter: f64,
    state: Arc<Mutex<MaskState>>,
    registry: FaceElementRegistry,
    shared_state: SharedFaceState,
    pixel_drawer: PixelDrawer,
}

impl ProtogenFace {
    fn new(audio_level: Arc<AudioLevel>, state: Arc<Mutex<MaskState>>) -> Self {
        let mut registry = FaceElementRegistry::new();

        // Register default elements
        registry.register(Box::new(DefaultEyes::new()));
        registry.register(Box::new(DefaultMouth::new(audio_level.clone())));
        registry.register(Box::new(DefaultNose));

        // Register alternative eye options
        registry.register(Box::new(HeartEyes));
        registry.register(Box::new(XEyes));
        registry.register(Box::new(OEyes));

        // Register accessories (commented out by default)
        // registry.register(Box::new(RunnyNose::new()));

        println!("✨ Registered {} face elements", registry.elements.len());
        println!("   Eyes: {}", registry.eyes_variants.join(", "));

        Self {
            time_counter: 0.0,
            state,
            registry,
            shared_state: SharedFaceState {
                mouth_opening: 0.0,
                eye_top: 9.0,
                eye_bottom: 1.45,
                blink_enabled: true,
            },
            pixel_drawer: PixelDrawer,
        }
    }

    // Cycle to the next eye variant
    fn cycle_eyes(&mut self) {
        self.registry.cycle_eyes();
    }

    // Handle button input for elements
    fn handle_element_button(&mut self, button: Button) -> bool {
        self.registry.handle_button(button, &mut self.shared_state)
    }

    fn render(&mut self, canvas: &mut LedCanvas) {
        self.time_counter += 1.0;

        // Get mask state
        let state = self.state.lock().unwrap();
        self.shared_state.blink_enabled = state.blink_enabled;
        let brightness = state.brightness;
        let palette = state.color_palette;

        // Handle manual mouth override
        if let Some(manual_mouth) = state.manual_mouth_override {
            self.shared_state.mouth_opening = manual_mouth;
        }
        drop(state);

        // Update all elements
        self.registry.update_all(&mut self.shared_state, 0.033); // ~30fps

        // Clear canvas
        canvas.clear();

        // Create render context
        let context = RenderContext {
            offset_x: 0.0, // Could add MPU head movement here
            offset_y: 0.0,
            time_counter: self.time_counter,
            brightness,
            palette,
        };

        // Render all elements through registry
        self.registry.render_all(canvas, &context, &self.shared_state, &self.pixel_drawer)
        }
}

// Implement CycleEyes trait for gamepad controls
impl CycleEyes for ProtogenFace {
    fn cycle_eyes(&mut self) {
        self.registry.cycle_eyes();
        let eyes_name = self.registry.get_active_eyes_name();
        println!("👁️  Eyes: {}", eyes_name);
    }
}

// Gamepad input handler

// Initialize microphone capture

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize audio capture
    let audio_level = Arc::new(AudioLevel::new());

    println!("Initializing microphone...");
    let _stream = match start_audio_capture(audio_level.clone()) {
        Ok(stream) => {
            println!("✅ Microphone initialized successfully!");
            Some(stream)
        }
        Err(e) => {
            eprintln!("⚠️  Warning: Could not initialize microphone: {}", e);
            eprintln!("Will use breathing animation only.");
            None
        }
    };

    // Initialize gamepad
    let mut gilrs = Gilrs::new().unwrap();
    let mask_state = Arc::new(Mutex::new(MaskState::new()));

    // Check for connected gamepads
    println!("\n🎮 Gamepad Status:");
    let mut gamepad_found = false;
    let mut gamepad_id = None;
    for (id, gamepad) in gilrs.gamepads() {
        println!("  Connected: {} (ID: {:?}, Power: {:?})", gamepad.name(), id, gamepad.power_info());
        println!("  Mapping: {:?}", gamepad.mapping_source());
        gamepad_found = true;
        gamepad_id = Some(id);
    }
    if !gamepad_found {
        println!("  ⚠️  No gamepad detected. Controls disabled.");
        println!("  Tip: Connect a Bluetooth gamepad and pair it before starting.");
        println!("  Debug: Check 'ls /dev/input/' and permissions");
    } else {
        println!("  ✅ Gamepad ready! Press any button to test...");
    }

    // Initialize LED matrix
    let mut options = LedMatrixOptions::new();
    options.set_rows(32);
    options.set_cols(64);
    options.set_chain_length(2);
    options.set_hardware_mapping("adafruit-hat");

    let matrix = LedMatrix::new(Some(options), None)?;
    let mut protogen = ProtogenFace::new(audio_level.clone(), mask_state.clone());

    println!("\n🚀 Starting animation loop...");
    println!("Microphone threshold: {}", SILENT_LIMIT);
    println!("Idle timeout: {} seconds", IDLE_TIMEOUT_SECS);
    println!("\n📋 Gamepad Controls:");
    println!("  A/X       - Toggle microphone mute");
    println!("  B/Circle  - Toggle manual breathing");
    println!("  Y/Triangle- Toggle blinking");
    println!("  X/Square  - Cycle color palette");
    println!("  D-Pad ↑↓  - Adjust brightness");
    println!("  D-Pad ←→  - Cycle eye styles");
    println!("  L Trigger - Open mouth (hold)");
    println!("  R Trigger - Close mouth (hold)");
    println!("  Start     - Reset to defaults\n");

    // Animation loop (run indefinitely)
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        loop {
            // Handle gamepad input (non-blocking)
            handle_gamepad_input(&mut gilrs, &mask_state, &mut protogen);

            let mut canvas = matrix.offscreen_canvas();
            protogen.render(&mut canvas);
            let _ = matrix.swap(canvas);

            thread::sleep(Duration::from_millis(33)); // ~30 FPS

            // Print status every few seconds
            if protogen.time_counter as u64 % 90 == 0 {
                let state = mask_state.lock().unwrap();
                let idle_secs = audio_level.seconds_since_audio();
                let current_level = audio_level.get_level();
                let mode = if state.mic_muted || state.manual_breathing {
                    "MANUAL"
                } else if idle_secs < IDLE_TIMEOUT_SECS {
                    "MIC"
                } else {
                    "BREATHING"
                };
                let eyes = protogen.registry.get_active_eyes_name();
                let mouth = protogen.shared_state.mouth_opening;
                println!("Mode: {} | Eyes: {} | Audio: {:.4} | Brightness: {:.0}% | Palette: {} | Mouth: {:.2}",
                         mode, eyes, current_level, state.brightness * 100.0, state.color_palette.name(), mouth);
            }
        }
    }));

    // Clear the display on exit (whether normal or panic)
    println!("\n🧹 Clearing display...");
    let mut canvas = matrix.offscreen_canvas();
    canvas.clear();
    let _ = matrix.swap(canvas);
    println!("✅ Display cleared. Goodbye!\n");

    // Propagate panic if one occurred
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }

    Ok(())
}