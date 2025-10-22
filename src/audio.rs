use std::sync::{Arc, Mutex};
use std::time::Instant;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

// Microphone constants (matching Arduino code)
pub const SILENT_LIMIT: f64 = 0.05; // Normalized audio threshold (0.0 to 1.0)

// Audio level tracker
pub struct AudioLevel {
    current_level: Arc<Mutex<f64>>,
    last_audio_time: Arc<Mutex<Instant>>,
}

impl AudioLevel {
    pub fn new() -> Self {
        Self {
            current_level: Arc::new(Mutex::new(0.0)),
            last_audio_time: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn update(&self, level: f64) {
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

    pub fn get_level(&self) -> f64 {
        self.current_level.lock().map(|l| *l).unwrap_or(0.0)
    }

    pub fn seconds_since_audio(&self) -> u64 {
        self.last_audio_time.lock()
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0)
    }
}

// Initialize microphone capture
pub fn start_audio_capture(audio_level: Arc<AudioLevel>) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
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
