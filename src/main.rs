// Module declarations
mod audio;
mod color;
mod elements;
mod face;
mod gamepad;
mod video;

use rpi_led_matrix::{LedMatrix, LedMatrixOptions, LedCanvas, LedColor};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::any::Any;
use gilrs::{Gilrs, Button};

// Re-export from modules
use audio::{AudioLevel, start_audio_capture, SILENT_LIMIT};
use color::ColorPalette;
use face::ProtogenFace;
use gamepad::{MaskState, handle_gamepad_input, ButtonTracker, VideoAction, print_control_mapping};
use video::VideoPlayer;

// Hardware constants
const PANEL_WIDTH: i32 = 64;
const PANEL_HEIGHT: i32 = 32;

// Microphone constants (matching Arduino code)
const MOUTH_MAX_OPENING: f64 = 6.0;
const IDLE_TIMEOUT_SECS: u64 = 30; // Switch to breathing after 30 seconds of silence

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

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
    let mut button_tracker = ButtonTracker::new();

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

    // Initialize video player
    let mut video_player = VideoPlayer::new("./videos");

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
    print_control_mapping();

    // Animation loop (run indefinitely - press Ctrl+C to stop)
    loop {
        // Handle gamepad input (non-blocking)
        handle_gamepad_input(&mut gilrs, &mask_state, &mut protogen, &mut button_tracker);

        // Handle video actions from gamepad
        {
            let mut state = mask_state.lock().unwrap();
            match state.video_action {
                VideoAction::PlayFirst => {
                    if video_player.play_first() {
                        state.video_mode = true;
                    }
                    state.video_action = VideoAction::None;
                }
                VideoAction::NextVideo => {
                    video_player.next_video();
                    state.video_action = VideoAction::None;
                }
                VideoAction::ExitVideo => {
                    video_player.stop();
                    state.video_mode = false;
                    state.video_action = VideoAction::None;
                }
                VideoAction::None => {}
            }
        }

        let mut canvas = matrix.offscreen_canvas();

        // Render based on mode
        let state = mask_state.lock().unwrap();
        if state.video_mode && video_player.is_playing() {
            // Video mode - render video frame (mirrored on both 64x32 panels)
            if let Some(frame) = video_player.next_frame(64, 32) {
                // Apply brightness
                let brightness = (state.brightness * 255.0) as u8;

                // Draw video frame mirrored on both panels
                for y in 0..32 {
                    for x in 0..64 {
                        let (r, g, b) = frame.get_pixel(x, y);
                        let r = ((r as u16 * brightness as u16) / 255) as u8;
                        let g = ((g as u16 * brightness as u16) / 255) as u8;
                        let b = ((b as u16 * brightness as u16) / 255) as u8;
                        let color = LedColor { red: r, green: g, blue: b };

                        // Draw on left panel
                        canvas.set(x as i32, y as i32, &color);
                        // Mirror on right panel
                        canvas.set((x + 64) as i32, y as i32, &color);
                    }
                }
            } else if video_player.has_ended() {
                // Video ended, return to face
                drop(state);
                let mut state = mask_state.lock().unwrap();
                state.video_mode = false;
                video_player.stop();
                println!("📺 Video ended, returning to protogen face");
            }
        } else {
            // Protogen face mode
            drop(state);
            protogen.render(&mut canvas);
        }

        let _ = matrix.swap(canvas);

        thread::sleep(Duration::from_millis(33)); // ~30 FPS
    }
}