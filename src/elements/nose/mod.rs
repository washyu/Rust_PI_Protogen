// Nose base trait
pub mod base;

// Individual nose implementations
mod default;

// Re-export the base trait and types
pub use base::{Nose, NosePosition};

// Re-export all nose implementations
pub use default::DefaultNose;

/// Get all available nose types as boxed trait objects
/// This allows the registry to auto-discover all nose implementations
pub fn get_all_nose_types() -> Vec<Box<dyn Nose>> {
    vec![
        Box::new(DefaultNose::new()),
    ]
}
