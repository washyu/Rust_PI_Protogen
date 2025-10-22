# Protogen LED Matrix Display

Rust implementation of a protogen face animation for Raspberry Pi with HUB75 LED matrix panels. Features real-time audio-reactive mouth animation with USB microphone input and idle breathing effects.

## Hardware Requirements

- Raspberry Pi Zero 2W (or other Pi models)
- 2x 64x32 HUB75 LED matrix panels (chained)
- Adafruit RGB Matrix HAT or Bonnet
- USB microphone (optional - will fall back to breathing animation if not available)
- Bluetooth gamepad (optional - for real-time control)
- 5V power supply (adequate for LED panels - typically 4A+ recommended)

## Software Prerequisites

### 1. Install Rust

If you don't have Rust installed on your Raspberry Pi:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. Install System Dependencies

```bash
sudo apt-get update
sudo apt-get install -y \
    libasound2-dev \
    pkg-config \
    build-essential \
    git
```

**Required libraries:**
- `libasound2-dev` - ALSA audio library (for USB microphone support)
- `pkg-config` - Helps find installed libraries during compilation
- `build-essential` - C/C++ compiler and tools (needed for rpi-led-matrix)
- `git` - Version control (for cloning rpi-rgb-led-matrix if needed)

### 3. Enable LED Matrix (Disable Audio PWM)

The LED matrix uses hardware that conflicts with the Pi's audio output. Edit `/boot/config.txt`:

```bash
sudo nano /boot/config.txt
```

Add or modify these lines:

```
# Disable audio (conflicts with LED matrix)
dtparam=audio=off

# Optional: Disable Bluetooth to free up resources on Pi Zero 2W
dtoverlay=disable-bt
```

Reboot after making changes:
```bash
sudo reboot
```

### 4. Pairing a Bluetooth Gamepad (Optional)

To use gamepad controls, pair your Bluetooth controller before starting the application:

```bash
sudo bluetoothctl
```

In the Bluetooth control interface:
```
power on
agent on
default-agent
scan on
```

Wait for your gamepad to appear, then:
```
pair XX:XX:XX:XX:XX:XX    # Replace with your gamepad's MAC address
connect XX:XX:XX:XX:XX:XX
trust XX:XX:XX:XX:XX:XX   # Auto-connect on boot
exit
```

**Supported gamepads:** PS4, PS5, Xbox One, Xbox Series, Nintendo Switch Pro, 8BitDo, and most generic Bluetooth controllers.

## Building the Project

```bash
cd ~/projects/led_matrix_test
cargo build --release
```

The first build will take 10-20 minutes on a Pi Zero 2W as it compiles all dependencies.

## Running the Application

The application requires root privileges to access GPIO pins:

```bash
sudo ./target/release/pi_mask_test
```

### Expected Output

```
Initializing microphone...
Using audio input device: USB Audio Device
Audio config: SupportedStreamConfig { ... }
Microphone initialized successfully!
Starting animation loop...
Microphone threshold: 0.05
Idle timeout: 30 seconds
Mode: MIC | Audio: 0.0234 | Idle: 2s | Mouth: 1.20
Mode: MIC | Audio: 0.1523 | Idle: 0s | Mouth: 4.50
Mode: BREATHING | Audio: 0.0012 | Idle: 31s | Mouth: 2.80
```

## Gamepad Controls

Control your protogen mask in real-time with a Bluetooth gamepad:

| Button | Function | Description |
|--------|----------|-------------|
| **A / X (PlayStation)** | Toggle Mic Mute | Force breathing mode even with audio input |
| **B / Circle** | Toggle Manual Breathing | Override auto-idle breathing |
| **Y / Triangle** | Toggle Blinking | Enable/disable eye blinks |
| **X / Square** | Cycle Color Palette | Switch between Forest, Fire, Ocean, Purple, Rainbow |
| **D-Pad Up** | Increase Brightness | +10% brightness (max 100%) |
| **D-Pad Down** | Decrease Brightness | -10% brightness (min 10%) |
| **D-Pad Left/Right** | Cycle Eye Style | Switch between Default, Heart, X, and O eyes |
| **L Trigger** | Open Mouth | Manually open mouth (hold) |
| **R Trigger** | Close Mouth | Manually close mouth (hold) |
| **Start** | Reset to Defaults | Reset all settings |

### Color Palettes

- **Forest (Green)** - Default green protogen look
- **Fire (Red/Orange)** - Warm red and orange tones
- **Ocean (Blue/Cyan)** - Cool blue aquatic colors
- **Purple/Pink** - Purple and magenta hues
- **Rainbow** - Multi-color cycling effect

### Control Feedback

Button presses provide console feedback:
```
🎤 Microphone MUTED
🎨 Color: Fire (Red/Orange)
🔆 Brightness: 80%
😮 Mouth: OPEN (manual)
🔄 Reset to defaults
```

## Configuration

### Audio Sensitivity

Adjust the microphone sensitivity by editing `src/main.rs`:

```rust
const SILENT_LIMIT: f64 = 0.05; // Increase for less sensitivity, decrease for more
```

- **Too sensitive?** Increase to `0.1` or higher
- **Not sensitive enough?** Decrease to `0.02` or lower

### Idle Timeout

Change how long before switching to breathing animation:

```rust
const IDLE_TIMEOUT_SECS: u64 = 30; // Seconds of silence before breathing mode
```

### LED Matrix Configuration

If using different panel configuration, edit `main()`:

```rust
options.set_rows(32);           // Panel height
options.set_cols(64);            // Panel width
options.set_chain_length(2);     // Number of panels chained
options.set_hardware_mapping("adafruit-hat"); // Or "adafruit-hat-pwm", "regular", etc.
```

### Color Palette

Change colors by editing the `get_shimmer_color()` function. Current palette is green (ForestColors). You can modify the RGB values:

```rust
let colors = [
    (0, 64, 0),       // Dark green
    (0, 128, 32),     // Forest green
    (32, 160, 64),    // Medium green
    // ... add more colors
];
```

## Features

### Audio-Reactive Mouth Animation
- Real-time USB microphone input
- RMS (Root Mean Square) amplitude detection
- Threshold-based mouth opening (matches original Arduino behavior)
- Opens mouth when audio detected, closes when silent

### Idle Breathing Animation
- Automatically activates after 30 seconds of silence
- Smooth sine wave breathing effect
- Seamless transition between modes

### Bluetooth Gamepad Control
- Real-time control without SSH/terminal access
- Tactile button control - no need to look at controls
- Toggle mic mute, breathing, blinking
- Adjust brightness on the fly
- Cycle through 5 color palettes
- Manual mouth control
- Perfect for controlling while wearing the mask

### Eye Blinking
- Periodic eye blinks every ~10 seconds
- Smooth multi-frame animation
- Can be disabled via gamepad

### Display Effects
- Mirrored face rendering (symmetrical left/right)
- 5 color palettes (Forest, Fire, Ocean, Purple, Rainbow)
- Adjustable brightness (10% - 100%)
- Color shimmer effect synchronized with animation
- Anti-aliased edges for smooth appearance

## Troubleshooting

### No Microphone Detected
```
Warning: Could not initialize microphone: No input device available
Will use breathing animation only.
```
**Solution:** Check USB microphone is connected. List audio devices:
```bash
arecord -l
```

### Permission Denied on GPIO
```
Error: Permission denied
```
**Solution:** Run with sudo:
```bash
sudo ./target/release/pi_mask_test
```

### Matrix Doesn't Light Up
- Check power supply is adequate (5V 4A+ recommended)
- Verify ribbon cable connections
- Try different `hardware_mapping` option in code
- Check `/boot/config.txt` has `dtparam=audio=off`

### Build Fails - ALSA Not Found
```
error: The system library `alsa` required by crate `alsa-sys` was not found
```
**Solution:** Install ALSA development libraries:
```bash
sudo apt-get install libasound2-dev pkg-config
```

### Audio Level Always Zero
- Check microphone permissions
- Test microphone: `arecord -d 5 test.wav && aplay test.wav`
- Adjust `SILENT_LIMIT` threshold in code
- Verify USB microphone is selected as default input

### Gamepad Not Detected
```
No gamepad detected. Controls disabled.
```
**Solution:**
- Pair gamepad via Bluetooth (see section 4 above)
- Check connection: `sudo bluetoothctl devices`
- Reconnect if needed: `sudo bluetoothctl connect XX:XX:XX:XX:XX:XX`
- Some gamepads require pressing a pairing button

### Gamepad Buttons Not Responding
- Check battery level on gamepad
- Re-pair the device
- Try a different gamepad model
- Check `dmesg` for input device errors

## Running on Boot (Optional)

To start the protogen display automatically on boot, create a systemd service:

```bash
sudo nano /etc/systemd/system/protogen.service
```

Add:

```ini
[Unit]
Description=Protogen LED Display
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/home/shaun/projects/led_matrix_test
ExecStart=/home/shaun/projects/led_matrix_test/target/release/pi_mask_test
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable protogen.service
sudo systemctl start protogen.service
```

Check status:
```bash
sudo systemctl status protogen.service
```

## Performance

- Targets ~30 FPS (33ms frame time)
- Pi Zero 2W should handle this comfortably
- Audio processing runs in separate thread
- Status printed every ~3 seconds

## Extending with Custom Face Elements

The protogen mask uses a modular **Face Element Registry** system that makes it easy to add custom face components without modifying core code.

### Available Eye Styles

Cycle through these with **D-Pad Left/Right**:
- **Default Eyes** - Original blinking protogen eyes
- **Heart Eyes** - Cute heart-shaped eyes (no blinking)
- **X Eyes** - Dizzy/knocked-out expression
- **O Eyes** - Surprised/shocked wide-open eyes

### Creating Your Own Elements

Face elements are modular Rust structs that implement the `FaceElement` trait. Each element handles its own:
- Update logic (animation state)
- Rendering (drawing to canvas)
- Input handling (gamepad buttons)

#### Example: Creating Custom Eyes

```rust
struct StarEyes;

impl FaceElement for StarEyes {
    fn name(&self) -> &str { "Star Eyes" }
    fn category(&self) -> ElementCategory { ElementCategory::Eyes }

    fn update(&mut self, shared_state: &mut SharedFaceState, _dt: f64) {
        // Don't blink
        shared_state.eye_top = 9.0;
        shared_state.eye_bottom = 1.45;
    }

    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
              _shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        // Draw star shapes at eye positions
        // ... your rendering code here ...
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
```

#### Registering Your Element

In `src/main.rs`, add your element to the registry in `ProtogenFace::new()`:

```rust
// Register alternative eye options
registry.register(Box::new(DefaultEyes::new()));
registry.register(Box::new(HeartEyes));
registry.register(Box::new(XEyes));
registry.register(Box::new(OEyes));
registry.register(Box::new(StarEyes));  // <-- Add your custom element
```

### Element Categories

Elements are organized by category:
- **Eyes** - Eye styles (only one active at a time, cycle with D-Pad)
- **Mouth** - Mouth animation (handles microphone input)
- **Nose** - Nose rendering
- **Accessory** - Additional effects (blush, tears, runny nose, etc.)

Multiple accessories can be active simultaneously!

### Advanced: Accessory Elements

Create accessories that layer on top of the base face:

```rust
struct Blush {
    intensity: f64,
}

impl FaceElement for Blush {
    fn name(&self) -> &str { "Blush" }
    fn category(&self) -> ElementCategory { ElementCategory::Accessory }

    fn update(&mut self, _shared_state: &mut SharedFaceState, _dt: f64) {
        // Pulse blush intensity
        self.intensity = (self.intensity + 0.05).sin().abs();
    }

    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext,
              _shared_state: &SharedFaceState, draw_pixel_fn: &dyn DrawPixelFn) {
        // Draw pink circles on cheeks
        // ...
    }

    // Handle input to toggle blush on/off
    fn handle_button(&mut self, button: Button, _shared_state: &mut SharedFaceState) -> bool {
        match button {
            Button::Select => {
                println!("💖 Blush toggled!");
                true // Button handled
            }
            _ => false
        }
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
```

### Shared State

Elements can read and write to `SharedFaceState`:
- `mouth_opening` - Current mouth open amount (0.0 to 6.0)
- `eye_top` / `eye_bottom` - Eyelid positions
- `blink_enabled` - Whether blinking is active

### Render Context

Each frame provides `RenderContext` with:
- `offset_x` / `offset_y` - Head movement (can add MPU sensor here)
- `time_counter` - Animation time
- `brightness` - Current brightness setting
- `palette` - Active color palette

### Tips for Extension Developers

1. **Keep it simple** - Start with static shapes before adding animation
2. **Use the pixel drawer** - Call `draw_pixel_fn.draw()` for automatic mirroring
3. **Test incrementally** - Register your element and test rendering before adding logic
4. **Share your creations** - Custom elements are easy to share as separate files!

## Credits

Based on the Arduino ESP32 protogen code by the original creator. Ported to Rust for Raspberry Pi with added features:
- USB microphone support via CPAL
- Automatic idle detection
- Improved threading for audio capture
- Enhanced debugging output

## License

This project uses:
- `rpi-led-matrix` - Rust bindings for hzeller's rpi-rgb-led-matrix library
- `cpal` - Cross-platform audio library
- `FastLED`-inspired color palette system

## Further Customization

See the original Arduino code comments for detailed explanation of the coordinate system and geometry used for face rendering. The Rust implementation maintains the same mathematical model for compatibility.
