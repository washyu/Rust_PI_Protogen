use std::path::{Path, PathBuf};
use std::fs;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::format::{input, Pixel};
use ffmpeg_next::media::Type;
use ffmpeg_next::software::scaling::{context::Context, flag::Flags};
use ffmpeg_next::util::frame::video::Video;

/// Manages video playback and frame extraction
pub struct VideoPlayer {
    current_context: Option<VideoContext>,
    current_video_index: usize,
    video_files: Vec<PathBuf>,
    video_ended: bool,
}

struct VideoContext {
    ictx: ffmpeg::format::context::Input,
    decoder: ffmpeg::decoder::Video,
    scaler: Context,
    stream_index: usize,
}

impl VideoPlayer {
    /// Create a new VideoPlayer and scan the videos directory
    pub fn new(videos_dir: &str) -> Self {
        // Initialize FFmpeg
        ffmpeg::init().ok();

        let video_files = Self::scan_video_directory(videos_dir);

        if video_files.is_empty() {
            println!("⚠️  No video files found in {}", videos_dir);
        } else {
            println!("📹 Found {} video file(s) in {}", video_files.len(), videos_dir);
            for (i, file) in video_files.iter().enumerate() {
                println!("   [{}] {}", i, file.display());
            }
        }

        VideoPlayer {
            current_context: None,
            current_video_index: 0,
            video_files,
            video_ended: false,
        }
    }

    /// Scan directory for video files
    fn scan_video_directory(dir: &str) -> Vec<PathBuf> {
        let path = Path::new(dir);

        if !path.exists() {
            println!("📁 Creating videos directory: {}", dir);
            if let Err(e) = fs::create_dir_all(path) {
                println!("❌ Failed to create directory: {}", e);
                return Vec::new();
            }
        }

        let mut files = Vec::new();

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if matches!(ext.as_str(), "mp4" | "avi" | "mov" | "mkv" | "webm") {
                        files.push(path);
                    }
                }
            }
        }

        // Sort alphabetically for consistent ordering
        files.sort();
        files
    }

    /// Start playing the first video
    pub fn play_first(&mut self) -> bool {
        if self.video_files.is_empty() {
            println!("❌ No videos available to play");
            return false;
        }

        self.current_video_index = 0;
        self.load_video(0)
    }

    /// Skip to next video
    pub fn next_video(&mut self) -> bool {
        if self.video_files.is_empty() {
            return false;
        }

        self.current_video_index = (self.current_video_index + 1) % self.video_files.len();
        self.load_video(self.current_video_index)
    }

    /// Load a specific video by index
    fn load_video(&mut self, index: usize) -> bool {
        if index >= self.video_files.len() {
            return false;
        }

        let path = &self.video_files[index];
        println!("🎬 Loading video: {}", path.display());

        match self.open_video(path) {
            Ok(context) => {
                self.current_context = Some(context);
                self.video_ended = false;
                println!("✅ Video loaded successfully");
                true
            }
            Err(e) => {
                println!("❌ Failed to load video: {}", e);
                self.current_context = None;
                false
            }
        }
    }

    fn open_video(&self, path: &Path) -> Result<VideoContext, ffmpeg::Error> {
        let ictx = input(&path)?;

        let input_stream = ictx
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;
        let stream_index = input_stream.index();

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input_stream.parameters())?;
        let decoder = context_decoder.decoder().video()?;

        let scaler = Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGB24,
            64,
            32,
            Flags::BILINEAR,
        )?;

        Ok(VideoContext {
            ictx,
            decoder,
            scaler,
            stream_index,
        })
    }

    /// Get the next frame, scaled to matrix dimensions
    pub fn next_frame(&mut self, _width: usize, _height: usize) -> Option<VideoFrame> {
        let context = self.current_context.as_mut()?;

        loop {
            match context.ictx.packets().next() {
                Some((stream, packet)) => {
                    if stream.index() == context.stream_index {
                        match context.decoder.send_packet(&packet) {
                            Ok(_) => {
                                let mut decoded = Video::empty();
                                if context.decoder.receive_frame(&mut decoded).is_ok() {
                                    let mut rgb_frame = Video::empty();
                                    if context.scaler.run(&decoded, &mut rgb_frame).is_ok() {
                                        return Some(VideoFrame::from_frame(rgb_frame));
                                    }
                                }
                            }
                            Err(_) => continue,
                        }
                    }
                }
                None => {
                    // Try to flush decoder
                    context.decoder.send_eof().ok();
                    let mut decoded = Video::empty();
                    if context.decoder.receive_frame(&mut decoded).is_ok() {
                        let mut rgb_frame = Video::empty();
                        if context.scaler.run(&decoded, &mut rgb_frame).is_ok() {
                            return Some(VideoFrame::from_frame(rgb_frame));
                        }
                    }

                    // Video ended
                    self.video_ended = true;
                    println!("🏁 Video ended");
                    return None;
                }
            }
        }
    }

    /// Check if current video has ended
    pub fn has_ended(&self) -> bool {
        self.video_ended
    }

    /// Stop playback and clear decoder
    pub fn stop(&mut self) {
        self.current_context = None;
        self.video_ended = false;
        println!("⏹️  Video playback stopped");
    }

    /// Check if a video is currently loaded
    pub fn is_playing(&self) -> bool {
        self.current_context.is_some()
    }

    /// Get current video name
    pub fn current_video_name(&self) -> Option<String> {
        self.video_files
            .get(self.current_video_index)
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    }
}

/// A single video frame scaled to display dimensions
pub struct VideoFrame {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>, // RGB data
}

impl VideoFrame {
    /// Convert ffmpeg Video frame to our VideoFrame
    fn from_frame(frame: Video) -> Self {
        let width = frame.width() as usize;
        let height = frame.height() as usize;

        // Get RGB data from frame
        let data = frame.data(0).to_vec();

        VideoFrame {
            width,
            height,
            data,
        }
    }

    /// Get RGB color at pixel position
    pub fn get_pixel(&self, x: usize, y: usize) -> (u8, u8, u8) {
        if x >= self.width || y >= self.height {
            return (0, 0, 0);
        }

        let idx = (y * self.width + x) * 3;
        if idx + 2 < self.data.len() {
            (
                self.data[idx],
                self.data[idx + 1],
                self.data[idx + 2],
            )
        } else {
            (0, 0, 0)
        }
    }
}
