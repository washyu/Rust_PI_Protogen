# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust implementation of an animated protogen face for Raspberry Pi with dual 64x32 HUB75 LED matrix panels. This is a port of the original Arduino protogen mask code by m16ind, maintaining the mathematical rendering approach while adding modular architecture and new features.

**License:** Creative Commons Attribution 4.0 International (CC BY 4.0)
- Original Arduino code by m16ind (https://vk.com/m16ind)
- Rust port maintains attribution requirements per LICENSE file

## Development Workflow

### Remote Development on Raspberry Pi

This project is **developed remotely** on a Raspberry Pi. The typical workflow is:

```bash
# 1. Edit code locally in WSL/Linux
# 2. Copy to Pi via SCP
scp src/main.rs shaun@192.168.10.138:~/projects/led_matrix_test/src/

# 3. SSH to Pi and build
ssh shaun@192.168.10.138
cd ~/projects/led_matrix_test
cargo build --release

# 4. Run with sudo (GPIO requires root)
sudo ./target/release/pi_mask_test
```

**CRITICAL RULE:** Do NOT push to GitHub until code has been tested on the Pi. Always test locally first.

### Build Commands

```bash
# Debug build (faster compilation, slower runtime)
cargo build

# Release build (slow compilation, optimized runtime - use for actual deployment)
cargo build --release

# Run (requires sudo for GPIO access)
sudo ./target/release/pi_mask_test
```

**Note:** First build takes 10-20 minutes on Pi Zero 2W due to heavy dependencies (rpi-led-matrix, ffmpeg-next, gilrs).

## Architecture

### Module Structure

The codebase is organized into focused modules:

- **`main.rs`** - Face element registry system, rendering engine, main loop
- **`audio.rs`** - USB microphone capture using CPAL, RMS calculation
- **`color.rs`** - Color palettes (Forest, Fire, Ocean, Purple, Rainbow)
- **`gamepad.rs`** - Bluetooth gamepad input handling (gilrs), mask state management
- **`video.rs`** - Video playback using FFmpeg, frame extraction and scaling

### Face Element Registry System

The core architectural pattern is a **modular element registry** that allows swappable face components:

```rust
trait FaceElement {
    fn update(&mut self, shared_state: &mut SharedFaceState, dt: f64);
    fn render(&self, canvas: &mut LedCanvas, context: &RenderContext, ...);
}
```

**Element categories:**
- `Eyes` - Multiple variants (Default, Hearts, X, Circles) - user can cycle through
- `Mouth` - Audio-reactive or manual control
- `Nose` - Additional decorative elements
- `Accessory` - Blush, tears, etc.

**Key insight:** Only ONE eye variant is active at a time, but ALL elements of other categories render simultaneously. This allows mixing and matching face components.

### Rendering Pipeline

The rendering approach is **mathematically-based**, inherited from the Arduino code:

1. **Update phase** - All elements update their internal state based on audio, time, input
2. **Render phase** - Elements are rendered in order: Mouth → Nose → Eyes → Accessories
3. **Pixel drawing** - Each element draws using line equations and curves (not sprites)
4. **180° rotation + mirroring** - Display is rotated 180° and mirrored across both panels

**Important:** The face is rendered using parametric equations for lines and curves, not bitmap sprites. Each pixel is calculated mathematically per frame.

### Shared State Management

Elements communicate through `SharedFaceState`:

```rust
struct SharedFaceState {
    mouth_opening: f64,       // 0.0 to MOUTH_MAX_OPENING (6.0)
    eye_top: f64,             // Top eyelid position (angle_y_b in Arduino)
    eye_bottom: f64,          // Bottom eyelid position (angle_y_a in Arduino)
    blink_enabled: bool,
    manual_mouth_active: bool, // Flag to skip automatic mouth updates
}
```

**Critical pattern:** When manual control is active, set the flag BEFORE calling `update_all()` so elements can skip their automatic updates.

### Audio-Reactive Mouth

The mouth opening is driven by:
1. **Microphone input** - RMS audio level above `SILENT_LIMIT` opens mouth
2. **Idle timeout** - After 30 seconds of silence, switches to breathing animation
3. **Manual override** - Left analog trigger provides direct control (0.0 to 1.0)

Mouth state has momentum:
- Audio above threshold → mouth opens quickly (+1.5 per frame in original)
- Audio below threshold → mouth closes slowly (-0.8 per frame in original)

### Blinking Animation

Blinking follows the original Arduino timing:
- Waits 10 seconds after startup/last blink
- Advances through 6 frames closing (frames 0-5)
- Advances through 6 frames opening (frames 5-0)
- Total blink duration: ~12 frames at 30fps = ~0.4 seconds

**Implementation detail:** Blink animation advances EVERY frame (no delay), matching Arduino behavior.

### Video Playback

Video mode is a special state that:
- Replaces face rendering with video frames
- Scans `./videos/` directory for MP4, AVI, MOV, MKV, WEBM files
- Scales frames to 64x32 and mirrors across both panels
- Supports cycling through videos and returning to face mode

## Hardware-Specific Notes

### FFmpeg and Raspberry Pi Pixel Formats

**Critical dependency configuration:**

```toml
[dependencies.ffmpeg-next]
git = "https://github.com/zmwangx/rust-ffmpeg.git"
branch = "master"
default-features = false
features = ["codec", "format", "rpi", "software-scaling"]
```

The **`"rpi"` feature is REQUIRED** for Raspberry Pi. Without it, you'll get compilation errors about missing pixel format enum variants (`AV_PIX_FMT_SAND128`, `AV_PIX_FMT_RPI4_8`, etc.). This is because Raspberry Pi's FFmpeg includes hardware-specific pixel formats not in standard FFmpeg.

### System Dependencies

Required apt packages (see README.md for full list):
- `libasound2-dev` - ALSA for audio input
- `libudev-dev` - udev for gamepad support
- `libavcodec-dev`, `libavformat-dev`, `libavutil-dev`, `libavfilter-dev`, `libavdevice-dev` - FFmpeg libraries
- `libclang-dev`, `clang` - Required for FFmpeg bindings

### LED Matrix Configuration

- Hardware: 2x 64x32 HUB75 panels chained horizontally
- Display orientation: Rotated 180° (upside down in code, right-side up physically)
- Mirroring: Each panel shows mirrored content for symmetrical protogen face

## Common Patterns

### Adding a New Face Element

1. Create struct implementing `FaceElement` trait
2. Implement `update()` for animation logic
3. Implement `render()` using mathematical curves/lines
4. Register in `main()` using `registry.register(Box::new(YourElement::new()))`

### Gamepad Controls

Controls are defined in `gamepad.rs` and mapped in `MaskState`:
- Face buttons (A/B/X/Y) - Toggle features
- D-Pad - Brightness and eye cycling
- Analog trigger - Manual mouth control (reads event code 10 for left trigger)
- Start button - Short press vs long press detection (800ms threshold)

### Color Palette System

Colors are selected from discrete palettes using an index value:
```rust
let color = get_shimmer_color(color_index, brightness, palette);
```

The `color_index` typically increments spatially (each pixel/column) to create shimmer effects.

## Debugging Tips

- **Blinking issues:** Check debug output for "Blink frame UP/DOWN" messages
- **Gamepad not responding:** Check if controller is paired and trusted via `bluetoothctl`
- **Audio not working:** Verify USB mic with `arecord -l`, check `SILENT_LIMIT` threshold
- **Video playback errors:** Ensure FFmpeg libraries installed, check `videos/` directory exists
- **No display output:** Verify running with `sudo`, check `/boot/config.txt` for `dtparam=audio=off`

## Code Style Notes

- Overflow checks are disabled in both debug and release profiles (mathematical animations can overflow safely)
- F64 is used throughout for smooth animation (matches Arduino's `double` type)
- Constants match Arduino code: `MOUTH_MAX_OPENING = 6.0`, `SILENT_LIMIT`, etc.
- Coordinate system: Origin (0,0) is top-left before rotation

## Original Arduino Code Reference

**IMPORTANT:** This project aims to be a true Rust clone of the original Arduino protogen mask code. When implementing features or fixing bugs, always refer to the original Arduino implementation below to maintain accuracy.

The original code uses mathematical rendering with parametric equations for all face elements. The Rust port should maintain this approach rather than switching to sprite-based rendering.

### Original Arduino Code (PatternProto_ver9)

```cpp
// PatternProto_ver9 by m16ind
//...................................................................-*-....................................................................
//.............................................................-*+=====-....................................................................
//.................................................:*++......:=========....+======+..:=================+....................................
//............................................:=======+.....:=========+...-=======:.:=======+::----:*==*....................................
//...........................................*========*....*==========*...*======+..+======-................................................
//..........................................+=========*...+===++======:..-=======:.-==================:.....................................
//........................................-===++======:..+===::=======-..*=======..*==================:.....................................
//.......................................:===:.=======:-===+..+=======...=======*.-=======......======......................................
//......................................*===-.-=======*===*..:=======+..:=======-.*======+....-+=====*......................................
//.....................................+==*...:==========-...+=======*..+======+..*=================+.......................................
//...................................-===:....:********:....:*********----------....---------------.........................................
//..................................:==+....................................................................................................
//................................-+==+.....................................................................................................

/*
  Panel connection
  +----------+
  |  R1  G1  |    R1  -> IO25    G1 -> IO26
  |  B1  GND |    B1  -> IO27
  |  R2  G2  |    R2  -> IO14    G2 -> IO12
  |  B2  E   |    B2  -> IO13     E -> N/A (required for 1/32 scan panels, like 64x64. Any available pin would do, i.e. IO32 )
  |   A  B   |    A   -> IO23     B -> IO19
  |   C  D   |    C   -> IO05     D -> IO17
  | CLK  LAT |    CLK -> IO16   LAT -> IO 4
  |  OE  GND |    OE  -> IO15   GND -> ESP32 GND
  +----------+
*/

#include <ESP32-HUB75-MatrixPanel-I2S-DMA.h>
#include <FastLED.h>
#include <Wire.h>

#define debug 1 //not a zero for debug mode

#define PANEL_WIDTH 64
#define PANEL_HEIGHT 32
#define PANELS_NUMBER 2
#define PIN_E 32
#define PANE_WIDTH PANEL_WIDTH * PANELS_NUMBER
#define PANE_HEIGHT PANEL_HEIGHT
#define MIC_PORT 33

int MOUTH_MAX_OPENING = 6;
int MIC_VALUE;
double mouth;
int MIC_MIN = 4000; //just a big number, don't touch
int SILENT_LIMIT = 1650;  //750 or 1650 for me (from 0 to 4096)

double bright = 255;
double dryg = 3000;

int16_t AcX, AcY, AcZ, Tmp, GyX, GyY, GyZ;
double g_ac = 5; //gravity compensation for mpu
const int MPU_ADDR = 0x68;
#define MPU_SDA 21
#define MPU_SCL 22

int sec = 0;
int blink_frame = 0;
int blink_flag = 1;

double reel_sin, reel_cos;

unsigned long fps_timer = 0, time_counter = 0, fps = 0;

MatrixPanel_I2S_DMA* dma_display = nullptr;
CRGB currentColor;
// HeatColors_p LavaColors_p RainbowColors_p RainbowStripeColors_p CloudColors_p OceanColors_p ForestColors_p PartyColors_p   -   ready palettes
CRGBPalette16 what(CRGB::Black, CRGB::Blue, CRGB::Aqua, CRGB::White);
CRGBPalette16 redPalette = CRGBPalette16(CRGB::Orange, CRGB::OrangeRed, CRGB::Red, CRGB::Crimson, CRGB::Crimson, CRGB::Tomato, CRGB::Tomato, CRGB::Coral, CRGB::Orange, CRGB::OrangeRed, CRGB::Red, CRGB::Crimson, CRGB::Crimson, CRGB::Tomato, CRGB::Tomato, CRGB::Coral);
CRGBPalette16 violetPalette = CRGBPalette16(CRGB::Indigo, CRGB::Indigo, CRGB::Amethyst, CRGB::Green, CRGB::Lime, CRGB::Lime, CRGB::White, CRGB::Fuchsia, CRGB::Fuchsia, CRGB::Indigo, CRGB::Indigo, CRGB::Amethyst, CRGB::Orchid, CRGB::Orchid, CRGB::Fuchsia, CRGB::Fuchsia);

void setup()
{
  HUB75_I2S_CFG mxconfig;
  mxconfig.mx_height = PANEL_HEIGHT;      // we have 64 pix heigh panels
  mxconfig.chain_length = PANELS_NUMBER;  // we have 2 panels chained
  mxconfig.gpio.e = PIN_E;                // we MUST assign pin e to some free pin on a board to drive 64 pix height panels with 1/32 scan
  dma_display = new MatrixPanel_I2S_DMA(mxconfig);
  dma_display->begin();
  fps_timer = millis();

  if(debug) Serial.begin(115200);

  Wire.begin(MPU_SDA, MPU_SCL, 100000); // sda, scl, clock speed
  Wire.beginTransmission(MPU_ADDR);
  Wire.write(0x6B);  // PWR_MGMT_1 register
  Wire.write(0);     // set to zero (wakes up the MPU−6050)
  Wire.endTransmission(true);
  pinMode(MIC_PORT, INPUT);

  dma_display->setBrightness8(255);
}

void drawPixel(int bright_f, int cvet, int x, int y)
{
  currentColor = ColorFromPalette(ForestColors_p, cvet, bright_f);
  dma_display->drawPixelRGB888(x, y, currentColor.r, currentColor.g, currentColor.b);
  //currentColor = ColorFromPalette(redPalette, cvet, bright_f);                      //for two colors - one at left panel and another at right
  dma_display->drawPixelRGB888(PANE_WIDTH + 1 - x, y, currentColor.r, currentColor.g, currentColor.b);
}

void MicParsing(double &cord_m_b_y, double &cord_m_d_y, double &cord_m_f_y, double &angle_m_b)
{
  MIC_VALUE = analogRead(MIC_PORT);
  if (MIC_VALUE < MIC_MIN) MIC_MIN = MIC_VALUE;
  if (MIC_VALUE - SILENT_LIMIT > 0) mouth += 1.5;
  if (MIC_VALUE - SILENT_LIMIT < 0) mouth -= 0.8;
  if (mouth >= MOUTH_MAX_OPENING) mouth = MOUTH_MAX_OPENING;
  if (mouth <= 0) mouth = 0;

  cord_m_b_y += mouth/2;
  cord_m_d_y -= mouth;
  cord_m_f_y -= mouth*1.3;
  angle_m_b -= mouth/10;
}

void MPUParsing(double &reel_sin, double &reel_cos, int time_counter)
{
  Wire.beginTransmission(MPU_ADDR);
  Wire.write(0x3B);
  Wire.endTransmission(false);
  Wire.requestFrom(MPU_ADDR, 14, true);
  AcX = Wire.read() << 8 | Wire.read();
  AcY = Wire.read() << 8 | Wire.read();
  AcZ = Wire.read() << 8 | Wire.read();

  reel_sin = sin((double)time_counter / 30) / 2 + (double)AcZ / dryg;
  reel_cos = cos((double)time_counter / 30) / 2 + (double)AcX / dryg - g_ac;
  Serial.printf_P("MPU value x: %f\n", reel_sin);
  Serial.printf_P("MPU value y: %f\n", reel_cos);
}

void Blinking(double &angle_y_a, double &angle_y_b)
{
  if (sec < 10)
  return;

  if (blink_frame == 0)
  {
    angle_y_a = 2;
    angle_y_b = 8;
  }
  if (blink_frame == 1)
  {
    angle_y_a = 3;
    angle_y_b = 7;
  }
  if (blink_frame == 2)
  {
    angle_y_a = 4;
    angle_y_b = 6;
  }
  if (blink_frame == 3)
  {
    angle_y_a = 5;
    angle_y_b = 5;
  }
  if (blink_frame == 4)
  {
    angle_y_a = 6;
    angle_y_b = 4;
  }
  if (blink_frame == 5)
  {
    angle_y_a = 7;
    angle_y_b = 0.1;
    blink_flag = 0;
  }
  if (blink_flag) blink_frame++;
  else blink_frame--;

  if (blink_frame == -1)
  {
    sec = 0;
    blink_frame = 0;
    blink_flag = 1;
  }
}

void loop()
{
  double angle_y_a = 1.45; //down
  double angle_y_b =  9.0; //top
  double angle_y_c = -0.6; //front

  double angle_m_a =  1.3;
  double angle_m_b =  1.9;
  double angle_m_c = -1.2;
  double angle_m_d = -1.2;
  double angle_m_e =  1.2;
  double angle_m_f =  1.2;
  double angle_m_g = -1.6;

  double cord_y_a_x =  0.0 + reel_sin;
  double cord_y_a_y = 25.0 + reel_cos;
  double cord_y_b_x =  2.0 + reel_sin;
  double cord_y_b_y = 31.0 + reel_cos;
  double cord_y_c_x = 10.0 + reel_sin;
  double cord_y_c_y =  0.0 + reel_cos;
  double cord_y_d_x = 18.0 + reel_sin;
  double cord_y_d_y = 24.0 + reel_cos;

  double cord_m_a_x =  7.0 + reel_sin;
  double cord_m_a_y = 31.0 + reel_cos;
  double cord_m_b_x =  7.0 + reel_sin;
  double cord_m_b_y = 18.0 + reel_cos;
  double cord_m_c_x =  0.0 + reel_sin;
  double cord_m_c_y =-32.0 + reel_cos;
  double cord_m_d_x =  0.0 + reel_sin;
  double cord_m_d_y =-37.0 + reel_cos;
  double cord_m_e_x =  0.0 + reel_sin;
  double cord_m_e_y = 57.0 + reel_cos;
  double cord_m_f_x =  0.0 + reel_sin;
  double cord_m_f_y = 52.0 + reel_cos;
  double cord_m_g_x =  0.0 + reel_sin;
  double cord_m_g_y = -2.0 + reel_cos;

  double cord_n_a_x = 56.0 + reel_sin;
  double cord_n_a_y = 27.0 + reel_cos;
  double cord_n_b_x = 53.0 + reel_sin;
  double cord_n_b_y = 23.0 + reel_cos;

  double y_a, y_b, y_c, y_d, m_a, m_b, m_c, m_d, m_e, m_f, m_g, n_a, n_b;
  double color_zero = (double)time_counter + (double)AcZ / dryg + (double)AcX / dryg;
  double color, color2;

  MicParsing(cord_m_b_y, cord_m_d_y, cord_m_f_y, angle_m_b);
  MPUParsing(reel_sin, reel_cos, time_counter);
  Blinking(angle_y_a, angle_y_b);

  for (int x = 1; x <= PANEL_WIDTH; x++)
  {
    color_zero += 5.0;
    color = color_zero;
    color2 = color_zero;  //

    y_a = (cord_y_a_x - x) / angle_y_a + cord_y_a_y;
    y_b = (cord_y_b_x - x) / angle_y_b + cord_y_b_y;
    y_c = (cord_y_c_x - x) / angle_y_c + cord_y_c_y;
    y_d = 0.8 * (x - cord_y_d_x) * (x - cord_y_d_x) + cord_y_d_y;

    m_a = (cord_m_a_x - x) / angle_m_a + cord_m_a_y;
    m_b = (cord_m_b_x - x) / angle_m_b + cord_m_b_y;
    m_c = (cord_m_c_x - x) / angle_m_c + cord_m_c_y;
    m_d = (cord_m_d_x - x) / angle_m_d + cord_m_d_y;
    m_e = (cord_m_e_x - x) / angle_m_e + cord_m_e_y;
    m_f = (cord_m_f_x - x) / angle_m_f + cord_m_f_y;
    m_g = (cord_m_g_x - x) / angle_m_g + cord_m_g_y;

    n_a = -0.5 * (x - cord_n_a_x) * (x - cord_n_a_x) + cord_n_a_y;
    n_b = -0.1 * (x - cord_n_b_x) * (x - cord_n_b_x) + cord_n_b_y;

    for (int y = 0; y <= PANE_HEIGHT; y++)
    {
      color += 5.0;
      color2 -= 3.0;

      if (y_a < y && y_b > y && y_c < y && y_d > y){
        if (y_a < y - 1.0 && y_b > y + 1.0 && y_c < y - 1.0 && y_d > y + 1.0){
          drawPixel(bright, color2, x, y);
          continue;
        }
        if (y_a > y - 1.0){
          drawPixel(bright * (y - y_a), color2, x, y);
          continue;
        }
        if (y_b < y + 1.0){
          drawPixel(bright * (y_b - y), color2, x, y);
          continue;
        }
        if (y_c > y - 1.0){
          drawPixel(bright * (y - y_c), color2, x, y);
          continue;
        }
        if (y_d < y + 1.0){
          drawPixel(bright * (y_d - y), color2, x, y);
          continue;
        }
      }
      if ((m_e > y && m_f < y && m_c > y) || (m_c > y && m_d < y && m_e > y && m_b < y) || (m_b < y && m_a > y && m_g > y && m_d < y))
      {
        if ((m_e >= y + 1.0 && m_f <= y - 1.0 && m_c >= y + 1.0) || (m_c >= y + 1.0 && m_d <= y - 1.0 && m_e >= y + 1.0 && m_b <= y - 1.0) || (m_a >= y + 1.0 && m_b <= y - 1.0 && m_g >= y + 1.0 && m_d <= y - 1.0)){
          drawPixel(bright, color, x, y);
          continue;
        }
        if (m_e < y + 1.0){
          drawPixel(bright * (m_e - y), color, x, y);
          continue;
        }
        if ((m_c < y + 1.0) && (m_a <= y)){
          drawPixel(bright * (m_c - y), color, x, y);
          continue;
        }
        if (m_a < y + 1.0 && m_a > y && m_c <= y){
          drawPixel(bright * (m_a - y), color, x, y);
          continue;
        }
        if (m_b > y - 1.0 && m_b < y && m_g > y + 1.0){
          drawPixel(bright * (y - m_b), color, x, y);
          continue;
        }
        if (m_d > y - 1.0 && m_d < y && m_f > y){
          drawPixel(bright * (y - m_d), color, x, y);
          continue;
        }
        if (m_f > y - 1.0 && m_f < y && m_d > y){
          drawPixel(bright * (y - m_f), color, x, y);
          continue;
        }
        if (m_g < y + 1.0 && m_g > y){
          drawPixel(bright * (m_g - y), color, x, y);
          continue;
        }
        if (m_f < y && m_f > y - 1.0 && m_d > y - 1.0){
          drawPixel(bright * (y - m_f), color, x, y);
          continue;
        }
        if (m_c > y && m_c < y + 1.0 && m_a < y + 1.0){
          drawPixel(bright * (m_c - y), color, x, y);
          continue;
        }
      }
      if ((n_b < y) && (n_a > y))
      {
        if ((n_b < y - 1.0) && (n_a > y + 1.0)){
          drawPixel(bright, color, x, y);
          continue;
        }
        if (n_b > y - 1.0){
          drawPixel(bright * (y - n_b), color, x, y);
          continue;
        }
        if (n_a < y + 1.0){
          drawPixel(bright * (n_a - y), color, x, y);
          continue;
        }
      }
      drawPixel(0, color, x, y);
    }
  }

  time_counter++;
  fps++;

  if (fps_timer + 1000 < millis())
  {
    if(debug)
    {
      Serial.printf_P("Mic Value: %d\n", analogRead(MIC_PORT));
      Serial.printf_P("Mic min: %d\n", MIC_MIN);
      Serial.printf_P("Effect fps: %d\n", fps);
    }
    fps_timer = millis();
    fps = 0;
    sec++;
  }
}

/*
  ASCII diagram showing face geometry:

                                                                                      _
                                                                                   /     g
                   /   \                                                      /           \
                                                                         /                 /
                /         \                                         /                 /
                                                                a                /
             /     /   \     \                             /                /
            e                 c                       /                b
          /     /         \     \                /                /
               f            d               /                /
       /     /               \     \   /                /
                                                   /
    /     /                     \             /

  Face elements mapping:
  - y_a, y_b, y_c, y_d = Eye boundaries (lines and curves)
  - m_a through m_g = Mouth shape (7 control lines)
  - n_a, n_b = Nose (parabolic curves)

  Variables naming convention:
  - cord_X_x/y = Coordinate points (x,y position)
  - angle_X = Slope/angle for line equations
  - X_a, X_b, etc = Calculated Y positions for each X column
*/
```

### Key Arduino Implementation Details to Maintain

1. **Blinking timing:** `sec` counter increments every second, blink starts at `sec >= 10`, advances `blink_frame` every loop iteration (not with delay)

2. **Mouth momentum:** Audio above threshold adds 1.5, below subtracts 0.8 per frame - creates natural opening/closing motion

3. **Mathematical rendering:** All face elements use parametric equations:
   - Eyes: Linear equations `y = (cord_x - x) / angle + cord_y` and parabolas
   - Mouth: 7 lines forming complex shape with boolean combinations
   - Nose: Parabolic curves with negative coefficients

4. **Anti-aliasing:** Edge pixels use fractional brightness based on distance from line (e.g., `bright * (y - y_a)`)

5. **Color increment:** `color_zero` increments by 5.0 per X column, then `color` increments by 5.0 and `color2` decrements by 3.0 per Y pixel - creates shimmer effect

6. **Coordinate offsets:** `reel_sin` and `reel_cos` add head movement (from MPU in Arduino, from time-based animation in Rust port)
