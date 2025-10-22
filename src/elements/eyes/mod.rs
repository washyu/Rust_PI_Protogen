// Eye base trait and configuration
pub mod base;

// Individual eye implementations
mod default;
mod heart;
mod circle;
mod cross;

// Re-export the base trait and types
pub use base::{Eye, EyePosition, BlinkConfig};

// Re-export all eye implementations
pub use default::DefaultEyes;
pub use heart::HeartEyes;
pub use circle::CircleEyes;
pub use cross::CrossEyes;

/// Get all available eye types as boxed trait objects
/// This allows the registry to auto-discover all eye implementations
pub fn get_all_eye_types() -> Vec<Box<dyn Eye>> {
    vec![
        Box::new(DefaultEyes::new()),
        Box::new(HeartEyes::new()),
        Box::new(CircleEyes::new()),
        Box::new(CrossEyes::new()),
    ]
}
