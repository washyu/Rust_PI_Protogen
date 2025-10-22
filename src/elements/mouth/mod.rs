// Mouth base trait
pub mod base;

// Individual mouth implementations
mod default;

// Re-export the base trait and types
pub use base::{Mouth, MouthMode};

// Re-export all mouth implementations
pub use default::DefaultMouth;

use crate::audio::AudioLevel;
use std::sync::Arc;

/// Get all available mouth types as boxed trait objects
/// This allows the registry to auto-discover all mouth implementations
pub fn get_all_mouth_types(audio_level: Arc<AudioLevel>) -> Vec<Box<dyn Mouth>> {
    vec![
        Box::new(DefaultMouth::new(audio_level)),
    ]
}
