use crate::utils;
use color_eyre::eyre;
use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

/// A builder for creating and running complex `FFmpeg` commands.
pub struct FfmpegCommand {
    inputs: Vec<(PathBuf, Option<f64>)>,
    filters: Vec<String>,
    maps: Vec<OsString>,
}

impl FfmpegCommand {
    /// Creates a new `FFmpeg` command with a primary input file.
    #[must_use]
    pub fn new(input: &Path) -> Self {
        Self {
            inputs: vec![(input.to_path_buf(), None)],
            filters: Vec::new(),
            maps: Vec::new(),
        }
    }

    /// Adds a new input file to be processed at a specific timestamp.
    /// Returns the input stream label (e.g., `[1:v]`).
    pub fn add_input_at_time(&mut self, timestamp: f64) -> String {
        let input_path = self.inputs[0].0.clone();
        self.inputs.push((input_path, Some(timestamp)));
        let input_idx = self.inputs.len() - 1;
        format!("[{input_idx}:v]")
    }

    /// Adds a `split` filter to create multiple identical streams from one input stream.
    pub fn add_split(&mut self, input_stream: &str, count: usize) -> Vec<String> {
        let labels: Vec<String> = (0..count)
            .map(|i| format!("[s{}_{i}]", self.filters.len()))
            .collect();
        self.filters
            .push(format!("{input_stream}split={count}{}", labels.join("")));
        labels
    }

    /// Adds an `asplit` filter for audio streams.
    pub fn add_asplit(&mut self, input_stream: &str, count: usize) -> Vec<String> {
        let labels: Vec<String> = (0..count)
            .map(|i| format!("[as{}_{i}]", self.filters.len()))
            .collect();
        self.filters
            .push(format!("{input_stream}asplit={count}{}", labels.join("")));
        labels
    }

    /// Adds a `scale` filter to resize a video stream.
    /// Returns the output stream label (e.g., `[out_0]`).
    ///
    /// Automatically ensures dimensions are even numbers.
    pub fn add_scale(&mut self, input_stream: &str, width: i32, height: i32) -> String {
        let sanitize = |val: i32| {
            if val <= 0 {
                // -2 tells FFmpeg: "keep aspect ratio and ensure divisible by 2"
                -2
            } else if val % 2 != 0 {
                val + 1 // Round up to even
            } else {
                val
            }
        };

        let w = sanitize(width);
        let h = sanitize(height);

        let out_label = format!("[out_{}]", self.filters.len());
        self.filters
            .push(format!("{input_stream}scale={w}:{h}{out_label}"));
        out_label
    }

    /// Maps a filtered stream to a single-frame image output file.
    pub fn map_still_output(&mut self, stream_label: &str, out_path: &Path) {
        self.maps.extend([
            "-map".into(),
            stream_label.into(),
            "-frames:v".into(),
            "1".into(),
            utils::path_to_os_string(out_path),
        ]);
    }

    /// Maps video and audio streams to a transcoded video output file.
    pub fn map_video_output(
        &mut self,
        video_stream: &str,
        audio_stream: Option<&str>,
        crf_quality: u64,
        out_path: &Path,
    ) {
        let mut map_args: Vec<OsString> = vec!["-map".into(), video_stream.into()];

        if let Some(a_stream) = audio_stream {
            map_args.extend([
                "-map".into(),
                a_stream.into(),
                "-c:a".into(),
                "libopus".into(),
                "-b:a".into(),
                "64k".into(),
            ]);
        } else {
            map_args.push("-an".into());
        }

        map_args.extend([
            "-c:v".into(),
            "libvpx-vp9".into(),
            "-crf".into(),
            crf_quality.to_string().into(),
            "-b:v".into(),
            "0".into(),
            utils::path_to_os_string(out_path),
        ]);

        self.maps.extend(map_args);
    }

    /// Builds and runs the `FFmpeg` command.
    pub async fn run(self) -> Result<()> {
        if self.maps.is_empty() {
            return Ok(()); // Nothing to do
        }

        let mut args: Vec<OsString> = vec!["-y".into()];
        for (path, timestamp) in &self.inputs {
            if let Some(ts) = timestamp {
                args.extend(["-ss".into(), ts.to_string().into()]);
            }
            args.extend(["-i".into(), utils::path_to_os_string(path)]);
        }

        if !self.filters.is_empty() {
            args.push("-filter_complex".into());
            args.push(self.filters.join(";").into());
        }

        args.extend(self.maps);
        run_ffmpeg(&args).await
    }
}

async fn run_ffmpeg<S: AsRef<OsStr> + Send + Sync>(args: &[S]) -> Result<()> {
    let output = Command::new("ffmpeg")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("failed to run ffmpeg")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("ffmpeg failed: {}", stderr.trim());
    }
    Ok(())
}

#[derive(Deserialize)]
struct FfprobeOutput {
    streams: Vec<StreamInfo>,
    format: FormatInfo,
}

#[derive(Deserialize)]
struct StreamInfo {
    codec_type: String,
    duration: Option<String>,
}

#[derive(Deserialize)]
struct FormatInfo {
    duration: Option<String>,
}

#[derive(Debug)]
pub struct VideoMetadata {
    pub duration: f64,
    pub has_audio: bool,
}

/// Gets metadata (duration and presence of audio) using ffprobe.
pub async fn get_video_metadata(video_path: &Path) -> Result<VideoMetadata> {
    let video_path_str = utils::path_to_os_string(video_path)
        .into_string()
        .map_err(|_| eyre::eyre!("ffprobe video path is not valid UTF-8"))?;

    let args = &[
        "-v",
        "quiet",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
        &video_path_str,
    ];

    let output = Command::new("ffprobe")
        .args(args)
        .output()
        .await
        .context("failed to run ffprobe")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("ffprobe failed: {}", stderr.trim());
    }

    let data: FfprobeOutput = serde_json::from_slice(&output.stdout)?;

    // Fallback logic for duration:
    // 1. Try the format container duration.
    // 2. If missing, try the duration of the first stream that has one.
    // 3. If still missing, default to 0.0.
    let duration = data
        .format
        .duration
        .or_else(|| data.streams.iter().find_map(|s| s.duration.clone()))
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(0.0);

    let has_audio = data.streams.iter().any(|s| s.codec_type == "audio");

    Ok(VideoMetadata {
        duration,
        has_audio,
    })
}
