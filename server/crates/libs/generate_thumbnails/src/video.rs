use crate::ffmpeg::{FfmpegCommand, get_video_metadata};
use app_state::ThumbnailSettings;
use color_eyre::eyre::Result;
use std::path::Path;
use tokio::fs;

pub async fn generate_video_thumbnails(
    input: &Path,
    output_dir: &Path,
    config: &ThumbnailSettings,
) -> Result<()> {
    fs::create_dir_all(output_dir).await?;

    let meta = get_video_metadata(input).await?;
    let mut cmd = FfmpegCommand::new(input);

    generate_fixed_time_stills(&mut cmd, output_dir, config, meta.duration);
    generate_percentage_stills(&mut cmd, output_dir, config, meta.duration);
    generate_video_transcodes(&mut cmd, output_dir, config, meta.has_audio);

    cmd.run().await
}

fn generate_fixed_time_stills(
    cmd: &mut FfmpegCommand,
    output_dir: &Path,
    config: &ThumbnailSettings,
    duration: f64,
) {
    if config.heights.is_empty() {
        return;
    }

    let safe_ts = config
        .video_options
        .thumb_time
        .min((duration - 0.5).max(0.0));
    let input_stream = cmd.add_input_at_time(safe_ts);
    let split_streams = cmd.add_split(&input_stream, config.heights.len());

    for (i, &h) in config.heights.iter().enumerate() {
        let scaled_stream = cmd.add_scale(&split_streams[i], -2, h as i32);
        let out_path = output_dir.join(format!("{h}p.{}", config.thumbnail_extension));
        cmd.map_still_output(&scaled_stream, &out_path);
    }
}

fn generate_percentage_stills(
    cmd: &mut FfmpegCommand,
    output_dir: &Path,
    config: &ThumbnailSettings,
    duration: f64,
) {
    if config.video_options.percentages.is_empty() {
        return;
    }
    let target_h = config.video_options.height;
    let thumb_ext = &config.thumbnail_extension;
    let max_safe_ts = (duration - 1.0).max(0.0);

    for &pct in &config.video_options.percentages {
        let raw_ts = (pct as f64) / 100.0 * duration;
        // Clamp the timestamp.
        let ts = raw_ts.min(max_safe_ts);

        let input_stream = cmd.add_input_at_time(ts);
        let scaled_stream = cmd.add_scale(&input_stream, -2, target_h as i32);
        let out_path = output_dir.join(format!("{pct}_percent.{thumb_ext}"));
        cmd.map_still_output(&scaled_stream, &out_path);
    }
}

fn generate_video_transcodes(
    cmd: &mut FfmpegCommand,
    output_dir: &Path,
    config: &ThumbnailSettings,
    has_audio: bool,
) {
    if config.video_options.transcode_outputs.is_empty() {
        return;
    }

    let main_video_stream = "[0:v]";
    let num_outputs = config.video_options.transcode_outputs.len();

    let v_streams = cmd.add_split(main_video_stream, num_outputs);

    let a_streams = if has_audio {
        Some(cmd.add_asplit("[0:a]", num_outputs))
    } else {
        None
    };

    for (i, hq_config) in config.video_options.transcode_outputs.iter().enumerate() {
        let h = hq_config.height;
        let scaled_v_stream = cmd.add_scale(&v_streams[i], -2, h as i32);
        let out_path = output_dir.join(format!("{h}p.{}", config.video_options.extension));

        // Get the specific audio stream label if available
        let audio_label = a_streams.as_ref().map(|labels| labels[i].as_str());

        cmd.map_video_output(&scaled_v_stream, audio_label, hq_config.quality, &out_path);
    }
}
