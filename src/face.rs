// Face management module
// Contains all face-related types, traits, and the main ProtogenFace struct

use std::any::Any;
use std::sync::{Arc, Mutex};
use rpi_led_matrix::LedCanvas;
use gilrs::Button;

use crate::audio::AudioLevel;
use crate::color::{ColorPalette, get_shimmer_color};
use crate::gamepad::{MaskState, CycleEyes};
use crate::elements;
use crate::{PANEL_WIDTH, PANEL_HEIGHT, MOUTH_MAX_OPENING};

// ============================================================================
// FACE ELEMENT SYSTEM
// ============================================================================

// Element categories for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElementCategory {
    Eyes,
    Mouth,
    Nose,
    Accessory, // Blush, tears, etc.
}

// Context passed to elements during rendering
pub struct RenderContext {
    pub offset_x: f64,
    pub offset_y: f64,
    pub time_counter: f64,
    pub brightness: f64,
    pub palette: ColorPalette,
}

// Shared state that elements can read/write
pub struct SharedFaceState {
    pub mouth_opening: f64,  // 0.0 to MOUTH_MAX_OPENING
    pub eye_top: f64,        // Top eyelid position
    pub eye_bottom: f64,     // Bottom eyelid position
    pub blink_enabled: bool,
    pub manual_mouth_active: bool,  // Skip mouth updates when true
}

// Trait for all face elements
pub trait FaceElement {
    fn name(&self) -> &str;
    fn category(&self) -> ElementCategory;
    fn description(&self) -> &str { "" }
    fn update(&mut self, shared_state: &mut SharedFaceState, dt: f64);
    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
              shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn);
    fn handle_button(&mut self, _button: Button, _shared_state: &mut SharedFaceState) -> bool {
        false
    }
    fn status(&self) -> String { String::new() }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// Helper trait for drawing pixels with state
pub trait DrawPixelFn {
    fn draw(&self, canvas: &mut LedCanvas, bright: f64, color_index: f64,
            x: i32, y: i32, brightness: f64, palette: ColorPalette);
}

// Pixel drawer implementation
pub struct PixelDrawer;

impl DrawPixelFn for PixelDrawer {
    fn draw(&self, canvas: &mut LedCanvas, bright_f: f64, color_index: f64,
            x: i32, y: i32, brightness: f64, palette: ColorPalette) {
        // Flip vertically only
        let flipped_y = PANEL_HEIGHT - 1 - y;

        if x < 0 || x >= PANEL_WIDTH || flipped_y < 0 || flipped_y >= PANEL_HEIGHT {
            return;
        }

        let adjusted_brightness = bright_f * brightness;
        let color = get_shimmer_color(color_index, adjusted_brightness, palette);

        // Draw on left panel (vertically flipped)
        canvas.set(x, flipped_y, &color);

        // Mirror on right panel (also vertically flipped)
        let mirror_x = (PANEL_WIDTH * 2) - 1 - x;
        if mirror_x >= PANEL_WIDTH && mirror_x < PANEL_WIDTH * 2 {
            canvas.set(mirror_x, flipped_y, &color);
        }
    }
}

// ============================================================================
// ELEMENT ADAPTERS
// ============================================================================

// Wrapper to adapt Eye trait to FaceElement trait
struct EyeElementAdapter {
    eye: Box<dyn elements::eyes::Eye>,
}

impl EyeElementAdapter {
    fn new(eye: Box<dyn elements::eyes::Eye>) -> Self {
        Self { eye }
    }
}

impl FaceElement for EyeElementAdapter {
    fn name(&self) -> &str {
        self.eye.name()
    }

    fn category(&self) -> ElementCategory {
        ElementCategory::Eyes
    }

    fn description(&self) -> &str {
        self.eye.description()
    }

    fn update(&mut self, shared_state: &mut SharedFaceState, dt: f64) {
        self.eye.update(shared_state, dt);
    }

    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
              shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        self.eye.draw(canvas, context, shared_state, draw_pixel_fn);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Wrapper to adapt Mouth trait to FaceElement trait
struct MouthElementAdapter {
    mouth: Box<dyn elements::mouth::Mouth>,
}

impl MouthElementAdapter {
    fn new(mouth: Box<dyn elements::mouth::Mouth>) -> Self {
        Self { mouth }
    }
}

impl FaceElement for MouthElementAdapter {
    fn name(&self) -> &str {
        self.mouth.name()
    }

    fn category(&self) -> ElementCategory {
        ElementCategory::Mouth
    }

    fn description(&self) -> &str {
        self.mouth.description()
    }

    fn update(&mut self, shared_state: &mut SharedFaceState, dt: f64) {
        self.mouth.update(shared_state, dt);
    }

    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
              shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        self.mouth.draw(canvas, context, shared_state, draw_pixel_fn);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Wrapper to adapt Nose trait to FaceElement trait
struct NoseElementAdapter {
    nose: Box<dyn elements::nose::Nose>,
}

impl NoseElementAdapter {
    fn new(nose: Box<dyn elements::nose::Nose>) -> Self {
        Self { nose }
    }
}

impl FaceElement for NoseElementAdapter {
    fn name(&self) -> &str {
        self.nose.name()
    }

    fn category(&self) -> ElementCategory {
        ElementCategory::Nose
    }

    fn description(&self) -> &str {
        self.nose.description()
    }

    fn update(&mut self, shared_state: &mut SharedFaceState, dt: f64) {
        self.nose.update(shared_state, dt);
    }

    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
              shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        self.nose.draw(canvas, context, shared_state, draw_pixel_fn);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ============================================================================
// FACE ELEMENT REGISTRY
// ============================================================================

struct FaceElementRegistry {
    elements: Vec<Box<dyn FaceElement>>,
    active_eyes_index: usize,
    eyes_variants: Vec<String>,
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
        if element.category() == ElementCategory::Eyes {
            self.eyes_variants.push(element.name().to_string());
        }
        self.elements.push(element);
    }

    fn update_all(&mut self, shared_state: &mut SharedFaceState, dt: f64) {
        for element in self.elements.iter_mut() {
            if element.category() == ElementCategory::Eyes {
                let eye_idx = self.eyes_variants.iter()
                    .position(|n| n == element.name());
                if let Some(ei) = eye_idx {
                    if ei != self.active_eyes_index {
                        continue;
                    }
                }
            }
            element.update(shared_state, dt);
        }
    }

    fn render_all(&self, canvas: &mut LedCanvas, context: &RenderContext,
                  shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        let order = [ElementCategory::Mouth, ElementCategory::Nose,
                     ElementCategory::Eyes, ElementCategory::Accessory];

        for category in &order {
            for element in self.elements.iter() {
                if element.category() != *category {
                    continue;
                }
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
        for element in &mut self.elements {
            if element.handle_button(button, shared_state) {
                return true;
            }
        }
        false
    }

    fn cycle_eyes_forward(&mut self) {
        if !self.eyes_variants.is_empty() {
            self.active_eyes_index = (self.active_eyes_index + 1) % self.eyes_variants.len();
        }
    }

    fn cycle_eyes_backward(&mut self) {
        if !self.eyes_variants.is_empty() {
            if self.active_eyes_index == 0 {
                self.active_eyes_index = self.eyes_variants.len() - 1;
            } else {
                self.active_eyes_index -= 1;
            }
        }
    }

    fn get_active_eyes_name(&self) -> String {
        self.eyes_variants.get(self.active_eyes_index)
            .cloned()
            .unwrap_or_else(|| "None".to_string())
    }
}

// ============================================================================
// PROTOGEN FACE
// ============================================================================

pub struct ProtogenFace {
    time_counter: f64,
    state: Arc<Mutex<MaskState>>,
    registry: FaceElementRegistry,
    shared_state: SharedFaceState,
    pixel_drawer: PixelDrawer,
}

impl ProtogenFace {
    pub fn new(audio_level: Arc<AudioLevel>, state: Arc<Mutex<MaskState>>) -> Self {
        let mut registry = FaceElementRegistry::new();

        // Auto-register all face element types from elements module
        for eye in elements::get_all_eye_types() {
            registry.register(Box::new(EyeElementAdapter::new(eye)));
        }

        for mouth in elements::get_all_mouth_types(audio_level.clone()) {
            registry.register(Box::new(MouthElementAdapter::new(mouth)));
        }

        for nose in elements::get_all_nose_types() {
            registry.register(Box::new(NoseElementAdapter::new(nose)));
        }

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
                manual_mouth_active: false,
            },
            pixel_drawer: PixelDrawer,
        }
    }

    pub fn render(&mut self, canvas: &mut LedCanvas) {
        self.time_counter += 1.0;

        // Get mask state
        let state = self.state.lock().unwrap();
        self.shared_state.blink_enabled = state.blink_enabled;
        let brightness = state.brightness;
        let palette = state.color_palette;
        let manual_mouth_mode = state.manual_mouth_mode;
        let mouth_analog_value = state.mouth_analog_value;

        self.shared_state.manual_mouth_active = manual_mouth_mode;
        drop(state);

        // Update all elements
        self.registry.update_all(&mut self.shared_state, 0.033);

        // Apply manual mouth control
        if manual_mouth_mode {
            self.shared_state.mouth_opening = mouth_analog_value * MOUTH_MAX_OPENING;
        }

        // Clear canvas
        canvas.clear();

        // Create render context
        let context = RenderContext {
            offset_x: 0.0,
            offset_y: 0.0,
            time_counter: self.time_counter,
            brightness,
            palette,
        };

        // Render all elements
        self.registry.render_all(canvas, &context, &self.shared_state, &self.pixel_drawer)
    }

    pub fn handle_element_button(&mut self, button: Button) -> bool {
        self.registry.handle_button(button, &mut self.shared_state)
    }

    pub fn get_active_eyes_name(&self) -> String {
        self.registry.get_active_eyes_name()
    }

    pub fn get_mouth_opening(&self) -> f64 {
        self.shared_state.mouth_opening
    }
}

// Implement CycleEyes trait for gamepad controls
impl CycleEyes for ProtogenFace {
    fn cycle_eyes_forward(&mut self) {
        self.registry.cycle_eyes_forward();
        let eyes_name = self.registry.get_active_eyes_name();
        println!("👁️  Eyes: {} (→)", eyes_name);
    }

    fn cycle_eyes_backward(&mut self) {
        self.registry.cycle_eyes_backward();
        let eyes_name = self.registry.get_active_eyes_name();
        println!("👁️  Eyes: {} (←)", eyes_name);
    }
}
