use rpi_led_matrix::{LedMatrix, LedMatrixOptions, LedColor, LedCanvas};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{HeapRb, traits::{Consumer, Observer, Producer, Split}};
use gilrs::{Gilrs, Event, Button, EventType};

const PANEL_WIDTH: i32 = 64;
const PANEL_HEIGHT: i32 = 32;

// Microphone constants (matching Arduino code)
const MOUTH_MAX_OPENING: f64 = 6.0;
const SILENT_LIMIT: f64 = 0.05; // Normalized audio threshold (0.0 to 1.0)
const IDLE_TIMEOUT_SECS: u64 = 30; // Switch to breathing after 30 seconds of silence

// Color palettes
#[derive(Debug, Clone, Copy, PartialEq)]
enum ColorPalette {
    Forest,      // Green
    Fire,        // Red/Orange
    Ocean,       // Blue/Cyan
    Purple,      // Purple/Pink
    Rainbow,     // Multi-color
}

impl ColorPalette {
    fn next(&self) -> Self {
        match self {
            ColorPalette::Forest => ColorPalette::Fire,
            ColorPalette::Fire => ColorPalette::Ocean,
            ColorPalette::Ocean => ColorPalette::Purple,
            ColorPalette::Purple => ColorPalette::Rainbow,
            ColorPalette::Rainbow => ColorPalette::Forest,
        }
    }

    fn name(&self) -> &str {
        match self {
            ColorPalette::Forest => "Forest (Green)",
            ColorPalette::Fire => "Fire (Red/Orange)",
            ColorPalette::Ocean => "Ocean (Blue/Cyan)",
            ColorPalette::Purple => "Purple/Pink",
            ColorPalette::Rainbow => "Rainbow",
        }
    }
}

// Mask control state
#[derive(Debug, Clone)]
struct MaskState {
    mic_muted: bool,           // Force breathing mode
    manual_breathing: bool,     // Override auto-idle
    brightness: f64,           // 0.0 to 1.0
    color_palette: ColorPalette,
    blink_enabled: bool,
    manual_mouth_override: Option<f64>, // Manual mouth control
}

// Audio level tracker
struct AudioLevel {
    current_level: Arc<Mutex<f64>>,
    last_audio_time: Arc<Mutex<Instant>>,
}

impl AudioLevel {
    fn new() -> Self {
        Self {
            current_level: Arc::new(Mutex::new(0.0)),
            last_audio_time: Arc::new(Mutex::new(Instant::now())),
        }
    }

    fn update(&self, level: f64) {
        if let Ok(mut current) = self.current_level.lock() {
            *current = level;
        }
        // Update last_audio_time if we're above threshold
        if level > SILENT_LIMIT {
            if let Ok(mut last_time) = self.last_audio_time.lock() {
                *last_time = Instant::now();
            }
        }
    }

    fn get_level(&self) -> f64 {
        self.current_level.lock().map(|l| *l).unwrap_or(0.0)
    }

    fn seconds_since_audio(&self) -> u64 {
        self.last_audio_time.lock()
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0)
    }
}

// Color palette for shimmer effect with multiple color schemes
fn get_shimmer_color(color_index: f64, brightness: f64, palette: ColorPalette) -> LedColor {
    let colors = match palette {
        ColorPalette::Forest => vec![
            (0, 64, 0), (0, 128, 32), (32, 160, 64),
            (64, 192, 96), (96, 224, 128), (128, 255, 160),
        ],
        ColorPalette::Fire => vec![
            (64, 16, 0), (128, 32, 0), (192, 64, 0),
            (255, 96, 0), (255, 128, 32), (255, 160, 64),
        ],
        ColorPalette::Ocean => vec![
            (0, 32, 64), (0, 64, 128), (0, 96, 192),
            (32, 128, 255), (64, 160, 255), (128, 192, 255),
        ],
        ColorPalette::Purple => vec![
            (64, 0, 64), (128, 0, 128), (160, 32, 160),
            (192, 64, 192), (224, 96, 224), (255, 128, 255),
        ],
        ColorPalette::Rainbow => vec![
            (255, 0, 0), (255, 128, 0), (255, 255, 0),
            (0, 255, 0), (0, 128, 255), (128, 0, 255),
        ],
    };

    let palette_index = (color_index.abs() as usize) % colors.len();
    let (r, g, b) = colors[palette_index];
    let bright_factor = (brightness / 255.0).clamp(0.0, 1.0);

    LedColor {
        red: (r as f64 * bright_factor) as u8,
        green: (g as f64 * bright_factor) as u8,
        blue: (b as f64 * bright_factor) as u8,
    }
}

struct ProtogenFace {
    time_counter: f64,
    mouth: f64,
    sec: i32,
    blink_frame: i32,
    blink_flag: bool,
    start_time: Instant,
    last_second: u64,
    audio_level: Arc<AudioLevel>,
    breathing_phase: f64, // For idle breathing animation
    state: Arc<Mutex<MaskState>>, // Gamepad-controllable state
}

impl ProtogenFace {
    fn new(audio_level: Arc<AudioLevel>, state: Arc<Mutex<MaskState>>) -> Self {
        Self {
            time_counter: 0.0,
            mouth: 0.0,
            sec: 0,
            blink_frame: 0,
            blink_flag: true,
            start_time: Instant::now(),
            last_second: 0,
            audio_level,
            breathing_phase: 0.0,
            state,
        }
    }

    fn update_animation(&mut self) {
        self.time_counter += 1.0;

        // Update second counter
        let current_second = self.start_time.elapsed().as_secs();
        if current_second != self.last_second {
            self.sec += 1;
            self.last_second = current_second;
        }

        // Get current state
        let state = self.state.lock().unwrap();

        // Check for manual mouth override first
        if let Some(manual_mouth) = state.manual_mouth_override {
            self.mouth = manual_mouth;
            drop(state); // Release lock early
            return;
        }

        let mic_muted = state.mic_muted;
        let manual_breathing = state.manual_breathing;
        drop(state); // Release lock

        // Determine animation mode
        let seconds_idle = self.audio_level.seconds_since_audio();
        let use_breathing = mic_muted || manual_breathing || (seconds_idle >= IDLE_TIMEOUT_SECS);

        if use_breathing {
            // Breathing animation (smooth sine wave)
            self.breathing_phase += 0.05;
            let breathing = (self.breathing_phase.sin() + 1.0) / 2.0; // 0.0 to 1.0
            let target_mouth = breathing * MOUTH_MAX_OPENING;

            // Smooth transition to breathing target
            if self.mouth < target_mouth {
                self.mouth += 0.1;
            } else {
                self.mouth -= 0.1;
            }
        } else {
            // Use microphone input (matching Arduino logic)
            let mic_level = self.audio_level.get_level();

            if mic_level > SILENT_LIMIT {
                self.mouth += 1.5; // Open mouth (Arduino: mouth += 1.5)
            } else {
                self.mouth -= 0.8; // Close mouth (Arduino: mouth -= 0.8)
            }
        }

        // Clamp mouth opening (Arduino logic)
        if self.mouth >= MOUTH_MAX_OPENING { self.mouth = MOUTH_MAX_OPENING; }
        if self.mouth <= 0.0 { self.mouth = 0.0; }
    }

    // Blinking function from Arduino code
    fn blinking(&mut self) -> (f64, f64) {
        let mut angle_y_a = 1.45;  // Default eye bottom
        let mut angle_y_b = 9.0;   // Default eye top

        // Check if blinking is disabled
        let blink_enabled = self.state.lock().unwrap().blink_enabled;
        if !blink_enabled || self.sec < 10 {
            return (angle_y_a, angle_y_b);
        }
        
        match self.blink_frame {
            0 => {
                angle_y_a = 2.0;
                angle_y_b = 8.0;
            }
            1 => {
                angle_y_a = 3.0;
                angle_y_b = 7.0;
            }
            2 => {
                angle_y_a = 4.0;
                angle_y_b = 6.0;
            }
            3 => {
                angle_y_a = 5.0;
                angle_y_b = 5.0;
            }
            4 => {
                angle_y_a = 6.0;
                angle_y_b = 4.0;
            }
            5 => {
                angle_y_a = 7.0;
                angle_y_b = 0.1;
                self.blink_flag = false;
            }
            _ => {}
        }
        
        if self.blink_flag {
            self.blink_frame += 1;
        } else {
            self.blink_frame -= 1;
        }
        
        if self.blink_frame == -1 {
            self.sec = 0;
            self.blink_frame = 0;
            self.blink_flag = true;
        }
        
        (angle_y_a, angle_y_b)
    }

    fn draw_pixel_mirrored(&self, canvas: &mut LedCanvas, bright_f: f64, color_index: f64, x: i32, y: i32) {
        // Rotate 180 degrees: new_x = width - x, new_y = height - y
        let rotated_x = PANEL_WIDTH - x;
        let rotated_y = PANEL_HEIGHT - 1 - y;

        if rotated_x < 0 || rotated_x >= PANEL_WIDTH || rotated_y < 0 || rotated_y >= PANEL_HEIGHT {
            return;
        }

        // Get current state for brightness and palette
        let state = self.state.lock().unwrap();
        let adjusted_brightness = bright_f * state.brightness;
        let palette = state.color_palette;
        drop(state);

        let color = get_shimmer_color(color_index, adjusted_brightness, palette);

        // Draw on left panel
        canvas.set(rotated_x, rotated_y, &color);

        // Mirror on right panel
        let mirror_x = (PANEL_WIDTH * 2) - 1 - rotated_x;
        if mirror_x >= PANEL_WIDTH && mirror_x < PANEL_WIDTH * 2 {
            canvas.set(mirror_x, rotated_y, &color);
        }
    }

    fn render(&mut self, canvas: &mut LedCanvas) {
        self.update_animation();
        canvas.clear();
        
        // Get blinking angles (modifies angle_y_a and angle_y_b)
        let (angle_y_a, angle_y_b) = self.blinking();
        let angle_y_c = -0.6;  // front (unchanged)
        
        let angle_m_a = 1.3;
        let angle_m_b = 1.9 - self.mouth / 10.0;  // mouth affected by breathing
        let angle_m_c = -1.2;
        let angle_m_d = -1.2;
        let angle_m_e = 1.2;
        let angle_m_f = 1.2;
        let angle_m_g = -1.6;

        // Coordinates (Arduino values, no accelerometer offset)
        let cord_y_a_x = 0.0;
        let cord_y_a_y = 25.0;
        let cord_y_b_x = 2.0;
        let cord_y_b_y = 31.0;
        let cord_y_c_x = 10.0;
        let cord_y_c_y = 0.0;
        let cord_y_d_x = 18.0;
        let cord_y_d_y = 24.0;
        
        let cord_m_a_x = 7.0;
        let cord_m_a_y = 31.0;
        let cord_m_b_x = 7.0;
        let cord_m_b_y = 18.0 + self.mouth / 2.0;  // mouth movement
        let cord_m_c_x = 0.0;
        let cord_m_c_y = -32.0;
        let cord_m_d_x = 0.0;
        let cord_m_d_y = -37.0 - self.mouth;       // mouth movement
        let cord_m_e_x = 0.0;
        let cord_m_e_y = 57.0;
        let cord_m_f_x = 0.0;
        let cord_m_f_y = 52.0 - self.mouth * 1.3;  // mouth movement
        let cord_m_g_x = 0.0;
        let cord_m_g_y = -2.0;

        let cord_n_a_x = 56.0;
        let cord_n_a_y = 27.0;
        let cord_n_b_x = 53.0;
        let cord_n_b_y = 23.0;

        let bright = 255.0;
        let color_zero = self.time_counter;

        // Main rendering loop (EXACT copy from Arduino)
        for x in 1..=PANEL_WIDTH {
            let mut color = color_zero + (x as f64) * 5.0;
            let mut color2 = color_zero + (x as f64) * 5.0;
            
            // Calculate boundaries (Arduino math) - now using blinking angles
            let y_a = (cord_y_a_x - x as f64) / angle_y_a + cord_y_a_y;
            let y_b = (cord_y_b_x - x as f64) / angle_y_b + cord_y_b_y;
            let y_c = (cord_y_c_x - x as f64) / angle_y_c + cord_y_c_y;
            let y_d = 0.8 * (x as f64 - cord_y_d_x).powi(2) + cord_y_d_y;
            
            let m_a = (cord_m_a_x - x as f64) / angle_m_a + cord_m_a_y;
            let m_b = (cord_m_b_x - x as f64) / angle_m_b + cord_m_b_y;
            let m_c = (cord_m_c_x - x as f64) / angle_m_c + cord_m_c_y;
            let m_d = (cord_m_d_x - x as f64) / angle_m_d + cord_m_d_y;
            let m_e = (cord_m_e_x - x as f64) / angle_m_e + cord_m_e_y;
            let m_f = (cord_m_f_x - x as f64) / angle_m_f + cord_m_f_y;
            let m_g = (cord_m_g_x - x as f64) / angle_m_g + cord_m_g_y;
            
            let n_a = -0.5 * (x as f64 - cord_n_a_x).powi(2) + cord_n_a_y;
            let n_b = -0.1 * (x as f64 - cord_n_b_x).powi(2) + cord_n_b_y;

            for y in 0..=PANEL_HEIGHT {
                color += 5.0;
                color2 -= 3.0;
                let y_f = y as f64;

                // Eye rendering (Arduino logic) - now affected by blinking
                if y_a < y_f && y_b > y_f && y_c < y_f && y_d > y_f {
                    let brightness = if y_a < y_f - 1.0 && y_b > y_f + 1.0 && 
                                        y_c < y_f - 1.0 && y_d > y_f + 1.0 {
                        bright
                    } else {
                        // Anti-aliasing (Arduino style)
                        if y_a > y_f - 1.0 {
                            bright * (y_f - y_a).max(0.0)
                        } else if y_b < y_f + 1.0 {
                            bright * (y_b - y_f).max(0.0)
                        } else if y_c > y_f - 1.0 {
                            bright * (y_f - y_c).max(0.0)
                        } else if y_d < y_f + 1.0 {
                            bright * (y_d - y_f).max(0.0)
                        } else {
                            bright
                        }
                    };
                    self.draw_pixel_mirrored(canvas, brightness, color2, x, y);
                    continue;
                }

                // Mouth rendering (Arduino logic)
                if (m_e > y_f && m_f < y_f && m_c > y_f) || 
                   (m_c > y_f && m_d < y_f && m_e > y_f && m_b < y_f) || 
                   (m_b < y_f && m_a > y_f && m_g > y_f && m_d < y_f) {
                    self.draw_pixel_mirrored(canvas, bright, color, x, y);
                    continue;
                }

                // Nose rendering (Arduino logic)  
                if (n_b < y_f) && (n_a > y_f) {
                    self.draw_pixel_mirrored(canvas, bright, color, x, y);
                    continue;
                }

                // Background (no pixel drawn)
            }
        }
    }
}

// Gamepad input handler
fn handle_gamepad_input(gilrs: &mut Gilrs, state: &Arc<Mutex<MaskState>>) {
    while let Some(Event { id, event, time: _ }) = gilrs.next_event() {
        // Debug: print all events
        println!("🎮 Event from gamepad {}: {:?}", id, event);

        match event {
            EventType::ButtonPressed(button, _) => {
                println!("🎮 Button pressed: {:?}", button);
                let mut s = state.lock().unwrap();
                match button {
                    // Face buttons
                    Button::South => {  // A/X button - Toggle mic mute
                        s.mic_muted = !s.mic_muted;
                        println!("🎤 Microphone {}", if s.mic_muted { "MUTED" } else { "ACTIVE" });
                    }
                    Button::East => {   // B/Circle button - Toggle manual breathing
                        s.manual_breathing = !s.manual_breathing;
                        println!("💨 Manual breathing {}", if s.manual_breathing { "ON" } else { "OFF" });
                    }
                    Button::North => {  // Y/Triangle button - Toggle blinking
                        s.blink_enabled = !s.blink_enabled;
                        println!("👁️  Blinking {}", if s.blink_enabled { "ON" } else { "OFF" });
                    }
                    Button::West => {   // X/Square button - Cycle color palette
                        s.color_palette = s.color_palette.next();
                        println!("🎨 Color: {}", s.color_palette.name());
                    }

                    // D-Pad for brightness
                    Button::DPadUp => {
                        s.brightness = (s.brightness + 0.1).min(1.0);
                        println!("🔆 Brightness: {:.0}%", s.brightness * 100.0);
                    }
                    Button::DPadDown => {
                        s.brightness = (s.brightness - 0.1).max(0.1);
                        println!("🔅 Brightness: {:.0}%", s.brightness * 100.0);
                    }

                    // Shoulder buttons for manual mouth control
                    Button::LeftTrigger | Button::LeftTrigger2 => {
                        s.manual_mouth_override = Some(MOUTH_MAX_OPENING); // Fully open
                        println!("😮 Mouth: OPEN (manual)");
                    }
                    Button::RightTrigger | Button::RightTrigger2 => {
                        s.manual_mouth_override = Some(0.0); // Fully closed
                        println!("😐 Mouth: CLOSED (manual)");
                    }

                    // Start/Select for reset
                    Button::Start => {
                        // Reset to defaults
                        s.mic_muted = false;
                        s.manual_breathing = false;
                        s.brightness = 1.0;
                        s.blink_enabled = true;
                        s.manual_mouth_override = None;
                        println!("🔄 Reset to defaults");
                    }

                    _ => {}
                }
            }
            EventType::ButtonReleased(button, _) => {
                match button {
                    Button::LeftTrigger | Button::LeftTrigger2 |
                    Button::RightTrigger | Button::RightTrigger2 => {
                        let mut s = state.lock().unwrap();
                        s.manual_mouth_override = None;
                        println!("🤖 Mouth: AUTO");
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

// Initialize microphone capture
fn start_audio_capture(audio_level: Arc<AudioLevel>) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or("No input device available")?;

    println!("Using audio input device: {}", device.name()?);

    let config = device.default_input_config()?;
    println!("Audio config: {:?}", config);

    let audio_level_clone = audio_level.clone();

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Calculate RMS amplitude (similar to Arduino analogRead)
                    let sum: f32 = data.iter().map(|&s| s * s).sum();
                    let rms = (sum / data.len() as f32).sqrt();
                    audio_level_clone.update(rms as f64);
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    // Normalize i16 to 0.0-1.0 range and calculate RMS
                    let sum: f32 = data.iter()
                        .map(|&s| {
                            let normalized = s as f32 / i16::MAX as f32;
                            normalized * normalized
                        })
                        .sum();
                    let rms = (sum / data.len() as f32).sqrt();
                    audio_level_clone.update(rms as f64);
                },
                |err| eprintln!("Audio stream error: {}", err),
                None,
            )?
        }
        _ => return Err("Unsupported sample format".into()),
    };

    stream.play()?;
    Ok(stream)
}

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
    let mask_state = Arc::new(Mutex::new(MaskState {
        mic_muted: false,
        manual_breathing: false,
        brightness: 1.0,
        color_palette: ColorPalette::Forest,
        blink_enabled: true,
        manual_mouth_override: None,
    }));

    // Check for connected gamepads
    println!("\n🎮 Gamepad Status:");
    let mut gamepad_found = false;
    let mut gamepad_id = None;
    for (id, gamepad) in gilrs.gamepads() {
        println!("  Connected: {} (ID: {:?}, Power: {:?})", gamepad.name(), id, gamepad.power_info());
        println!("  Mapping: {}", gamepad.mapping_source());
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
    println!("  L Trigger - Open mouth (hold)");
    println!("  R Trigger - Close mouth (hold)");
    println!("  Start     - Reset to defaults\n");

    // Animation loop (run indefinitely)
    loop {
        // Handle gamepad input (non-blocking)
        handle_gamepad_input(&mut gilrs, &mask_state);

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
            println!("Mode: {} | Audio: {:.4} | Brightness: {:.0}% | Palette: {} | Mouth: {:.2}",
                     mode, current_level, state.brightness * 100.0, state.color_palette.name(), protogen.mouth);
        }
    }
}