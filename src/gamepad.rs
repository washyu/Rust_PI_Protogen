use std::sync::{Arc, Mutex};
use std::time::Instant;
use gilrs::{Gilrs, Event, Button, EventType};
use crate::color::ColorPalette;
use crate::MOUTH_MAX_OPENING;

// Button press tracking for long press detection
pub struct ButtonTracker {
    start_pressed_at: Option<Instant>,
}

impl ButtonTracker {
    pub fn new() -> Self {
        Self {
            start_pressed_at: None,
        }
    }
}

// Mask control state
#[derive(Debug, Clone)]
pub struct MaskState {
    pub mic_muted: bool,           // Force breathing mode
    pub manual_breathing: bool,     // Override auto-idle
    pub brightness: f64,           // 0.0 to 1.0
    pub color_palette: ColorPalette,
    pub blink_enabled: bool,
    pub manual_mouth_override: Option<f64>, // Manual mouth control
    pub video_mode: bool,          // Video playback active
    pub video_action: VideoAction, // What to do with video
}

#[derive(Debug, Clone, PartialEq)]
pub enum VideoAction {
    None,
    PlayFirst,
    NextVideo,
    ExitVideo,
}

impl MaskState {
    pub fn new() -> Self {
        Self {
            mic_muted: false,
            manual_breathing: false,
            brightness: 1.0,
            color_palette: ColorPalette::Forest,
            blink_enabled: true,
            manual_mouth_override: None,
            video_mode: false,
            video_action: VideoAction::None,
        }
    }
}

// Gamepad input handler
pub fn handle_gamepad_input<T: CycleEyes>(gilrs: &mut Gilrs, state: &Arc<Mutex<MaskState>>,
                                          protogen: &mut T, button_tracker: &mut ButtonTracker) {
    while let Some(Event { id, event, time: _ }) = gilrs.next_event() {
        println!("🎮 Event from gamepad {}: {:?}", id, event);
        match event {
            EventType::ButtonPressed(button, _) => {
                println!("🎮 Button pressed: {:?}", button);

                // Track Start button press time for long press detection
                if button == Button::Start {
                    button_tracker.start_pressed_at = Some(Instant::now());
                }

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

                    // D-Pad for brightness and eye cycling
                    Button::DPadUp => {
                        s.brightness = (s.brightness + 0.1).min(1.0);
                        println!("🔆 Brightness: {:.0}%", s.brightness * 100.0);
                    }
                    Button::DPadDown => {
                        s.brightness = (s.brightness - 0.1).max(0.1);
                        println!("🔅 Brightness: {:.0}%", s.brightness * 100.0);
                    }
                    Button::DPadLeft | Button::DPadRight => {
                        drop(s); // Release lock before calling protogen
                        protogen.cycle_eyes();
                        return; // Exit early since lock is dropped
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

                    // Start button is handled on release to detect short vs long press
                    Button::Start => {
                        // Do nothing on press, wait for release
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
                    Button::Start => {
                        // Check press duration for short vs long press
                        if let Some(pressed_at) = button_tracker.start_pressed_at.take() {
                            let duration = pressed_at.elapsed();
                            let mut s = state.lock().unwrap();

                            // Long press = 800ms or more
                            if duration.as_millis() >= 800 {
                                // Long press: Exit video mode
                                if s.video_mode {
                                    s.video_action = VideoAction::ExitVideo;
                                    println!("📺 ⏹️  Long press: Exiting video mode");
                                }
                            } else {
                                // Short press: Play first video or skip to next
                                if s.video_mode {
                                    s.video_action = VideoAction::NextVideo;
                                    println!("📺 ⏭️  Short press: Next video");
                                } else {
                                    s.video_action = VideoAction::PlayFirst;
                                    println!("📺 ▶️  Short press: Start video playback");
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

// Trait for objects that can cycle eyes
pub trait CycleEyes {
    fn cycle_eyes(&mut self);
}
