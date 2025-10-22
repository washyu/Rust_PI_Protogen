// Face element modules
pub mod eyes;
pub mod mouth;
pub mod nose;
// TODO: Create accessories module
// pub mod accessories;

// Re-export eye module
pub use eyes::{Eye, EyePosition, BlinkConfig, get_all_eye_types};
pub use eyes::{DefaultEyes, HeartEyes, CircleEyes, CrossEyes};

// Re-export mouth module
pub use mouth::{Mouth, MouthMode, get_all_mouth_types};
pub use mouth::DefaultMouth;

// Re-export nose module
pub use nose::{Nose, NosePosition, get_all_nose_types};
pub use nose::DefaultNose;
