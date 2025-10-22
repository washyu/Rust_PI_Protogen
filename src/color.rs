use rpi_led_matrix::LedColor;

// Color palettes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorPalette {
    Forest,      // Green
    Fire,        // Red/Orange
    Ocean,       // Blue/Cyan
    Purple,      // Purple/Pink
    Rainbow,     // Multi-color
}

impl ColorPalette {
    pub fn next(&self) -> Self {
        match self {
            ColorPalette::Forest => ColorPalette::Fire,
            ColorPalette::Fire => ColorPalette::Ocean,
            ColorPalette::Ocean => ColorPalette::Purple,
            ColorPalette::Purple => ColorPalette::Rainbow,
            ColorPalette::Rainbow => ColorPalette::Forest,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ColorPalette::Forest => "Forest (Green)",
            ColorPalette::Fire => "Fire (Red/Orange)",
            ColorPalette::Ocean => "Ocean (Blue/Cyan)",
            ColorPalette::Purple => "Purple/Pink",
            ColorPalette::Rainbow => "Rainbow",
        }
    }
}

// Color palette for shimmer effect with multiple color schemes
pub fn get_shimmer_color(color_index: f64, brightness: f64, palette: ColorPalette) -> LedColor {
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

    // Smooth interpolation between colors
    let color_len = colors.len() as f64;
    let normalized_index = color_index.abs() % (color_len * 10.0); // Scale for smoother transitions
    let base_index = (normalized_index / 10.0) as usize % colors.len();
    let next_index = (base_index + 1) % colors.len();
    let blend = (normalized_index / 10.0) - (base_index as f64);

    let (r1, g1, b1) = colors[base_index];
    let (r2, g2, b2) = colors[next_index];

    // Linear interpolation between adjacent colors
    let r = r1 as f64 + (r2 as f64 - r1 as f64) * blend;
    let g = g1 as f64 + (g2 as f64 - g1 as f64) * blend;
    let b = b1 as f64 + (b2 as f64 - b1 as f64) * blend;

    let bright_factor = (brightness / 255.0).clamp(0.0, 1.0);

    LedColor {
        red: (r * bright_factor) as u8,
        green: (g * bright_factor) as u8,
        blue: (b * bright_factor) as u8,
    }
}
