use std::sync::Arc;
use rpi_led_matrix::LedCanvas;
use super::base::Mouth;
use crate::face::{RenderContext, DrawPixelFn, SharedFaceState};
use crate::{PANEL_WIDTH, PANEL_HEIGHT};
use crate::audio::{AudioLevel, SILENT_LIMIT};

const MOUTH_MAX_OPENING: f64 = 6.0;
const IDLE_TIMEOUT_SECS: u64 = 30;

/// Default audio-reactive mouth with breathing animation
#[derive(Clone)]
pub struct DefaultMouth {
    mouth_opening: f64,
    breathing_phase: f64,
    audio_level: Arc<AudioLevel>,
}

impl DefaultMouth {
    pub fn new(audio_level: Arc<AudioLevel>) -> Self {
        Self {
            mouth_opening: 0.0,
            breathing_phase: 0.0,
            audio_level,
        }
    }
}

impl Mouth for DefaultMouth {
    fn name(&self) -> &str {
        "Default Mouth"
    }

    fn description(&self) -> &str {
        "Audio-reactive mouth with microphone input and breathing animation"
    }

    fn update(&mut self, shared_state: &mut SharedFaceState, _dt: f64) {
        // Skip update if manual mouth control is active
        if shared_state.manual_mouth_active {
            return;
        }

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

    fn draw(&self, canvas: &mut LedCanvas, context: &RenderContext,
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

    fn clone_box(&self) -> Box<dyn Mouth> {
        Box::new(self.clone())
    }
}
