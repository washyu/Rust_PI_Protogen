# Video Files for Protogen Mask

Place your video files in this directory to play them on the LED matrix display.

## Supported Formats

- **MP4** (recommended)
- **AVI**
- **MOV**
- **MKV**
- **WEBM**

## Video Recommendations

For best results on the 128x32 LED matrix:

### Resolution
- Videos will be automatically scaled to 128x32 pixels
- Lower resolution videos (e.g., 640x480 or smaller) will load faster
- High resolution videos work but may have slower frame rates

### Frame Rate
- Target 30 FPS or lower for smooth playback
- Higher frame rates may drop frames on Raspberry Pi Zero 2W

### Duration
- Any length is supported
- Videos automatically return to protogen face when finished
- Consider short loops (5-15 seconds) for best effect

### Content Suggestions
- Simple animations work better than complex scenes
- High contrast content displays better on LEDs
- Avoid very dark scenes (won't show well on matrix)
- Test your video brightness - LED matrices are very bright!

## How to Use

### Adding Videos

1. Copy your video files to this directory:
   ```bash
   scp myvideo.mp4 pi@raspberrypi:~/workspace/pi_mask_test/videos/
   ```

2. Or use a USB drive:
   ```bash
   sudo mount /dev/sda1 /mnt
   cp /mnt/myvideo.mp4 ~/workspace/pi_mask_test/videos/
   ```

### Playing Videos

**From Protogen Face Mode:**
- Press **Start** button (short press) to play the first video

**During Video Playback:**
- Press **Start** button (short press) to skip to next video
- **Hold Start** button (long press, 800ms+) to exit back to protogen face
- Use **D-Pad Up/Down** to adjust brightness

### Video Playback Order

Videos play in alphabetical order. To control playback order, name your files:
```
01-intro.mp4
02-happy.mp4
03-excited.mp4
```

## Example: Converting Video for LED Matrix

Use ffmpeg to optimize videos for the LED matrix:

```bash
# Resize and optimize for LED matrix (128x32)
ffmpeg -i input.mp4 -vf scale=128:32 -r 30 -b:v 500k output.mp4

# Convert from GIF with loop
ffmpeg -i animated.gif -vf scale=128:32 -r 30 -pix_fmt yuv420p output.mp4

# Extract a short clip (first 10 seconds)
ffmpeg -i long_video.mp4 -t 10 -vf scale=128:32 -r 30 clip.mp4
```

## Troubleshooting

### "No video files found"
- Check that files are in the correct directory
- Verify file extensions are supported
- Use `ls -la ~/workspace/pi_mask_test/videos/` to list files

### Video won't play
- Check file isn't corrupted: `ffmpeg -i yourfile.mp4`
- Try converting to MP4 with above ffmpeg commands
- Check console output for error messages

### Playback is choppy
- Reduce video resolution
- Lower frame rate to 24 or 20 FPS
- Reduce video bitrate
- Use simpler video content

### Video is too dark/bright
- Adjust brightness with D-Pad Up/Down during playback
- Re-encode video with adjusted brightness:
  ```bash
  ffmpeg -i input.mp4 -vf eq=brightness=0.1 -r 30 output.mp4
  ```

## Sample Videos

Looking for test content? Try these free sources:

- **Sample Videos**: https://sample-videos.com/
- **Pexels Videos**: https://www.pexels.com/videos/ (free stock footage)
- **Pixabay Videos**: https://pixabay.com/videos/ (free videos)

Remember to resize them for the LED matrix using the ffmpeg commands above!

## Current Videos

(Empty directory - add your videos here!)
