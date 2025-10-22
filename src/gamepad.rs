use std::sync::{Arc, Mutex};
use gilrs::{Gilrs, Event, Button, EventType};
use crate::color::ColorPalette;
use crate::MOUTH_MAX_OPENING;

// Mask control state
#[derive(Debug, Clone)]
pub struct MaskState {
    pub mic_muted: bool,           // Force breathing mode
    pub manual_breathing: bool,     // Override auto-idle
    pub brightness: f64,           // 0.0 to 1.0
    pub color_palette: ColorPalette,
    pub blink_enabled: bool,
    pub manual_mouth_override: Option<f64>, // Manual mouth control
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
        }
    }
}

// Gamepad input handler
pub fn handle_gamepad_input<T: CycleEyes>(gilrs: &mut Gilrs, state: &Arc<Mutex<MaskState>>, protogen: &mut T) {
    while let Some(Event { id, event, time: _ }) = gilrs.next_event() {
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

// Trait for objects that can cycle eyes
pub trait CycleEyes {
    fn cycle_eyes(&mut self);
}
