use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use anyhow::{Context, Result, bail};
use crate::model::AudioFormat;

static CACHED_FFMPEG: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Creates a `std::process::Command` configured to execute without showing a console window on Windows.
pub fn silent_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW (0x08000000) prevents any black CMD or PowerShell window from popping up.
        cmd.creation_flags(0x08000000);
    }
    cmd
}

/// Finds the best available ffmpeg binary:
/// 1. System PATH ("ffmpeg")
/// 2. Local workspace binary if present in .private
pub fn find_ffmpeg() -> Option<PathBuf> {
    CACHED_FFMPEG.get_or_init(|| {
        // 1. Check system PATH
        let mut cmd = silent_command("ffmpeg");
        cmd.arg("-version");
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                return Some(PathBuf::from("ffmpeg"));
            }
        }

        // 2. Check local workspace reference if on Windows
        let reference_path = PathBuf::from(r"c:\Users\joshu\RustroverProjects\spotifydownloader\.private\Spotify-Downloader-main\Spotify Downloader\ffmpeg.exe");
        if reference_path.exists() {
            return Some(reference_path);
        }

        // 3. Check current dir or executable dir
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let local_ffmpeg = dir.join("ffmpeg.exe");
                if local_ffmpeg.exists() {
                    return Some(local_ffmpeg);
                }
            }
        }

        None
    }).clone()
}

/// Transcodes an input audio file (or raw audio) to the target format (MP3, M4A, or FLAC).
pub fn transcode_audio(
    input_file: &Path,
    output_file: &Path,
    format: AudioFormat,
) -> Result<()> {
    let ffmpeg_bin = find_ffmpeg().unwrap_or_else(|| PathBuf::from("ffmpeg"));

    let mut cmd = silent_command(&ffmpeg_bin);
    cmd.arg("-y"); // overwrite without asking
    cmd.arg("-i").arg(input_file);

    match format {
        AudioFormat::Mp3 => {
            cmd.args(["-c:a", "libmp3lame", "-b:a", "320k"]);
        }
        AudioFormat::M4a => {
            cmd.args(["-c:a", "alac"]);
        }
        AudioFormat::Flac => {
            cmd.args(["-c:a", "flac"]);
        }
    }

    cmd.arg(output_file);

    let output = cmd.output().with_context(|| {
        format!(
            "Failed to execute FFmpeg (looked for: {}). Please ensure FFmpeg is installed or available.",
            ffmpeg_bin.display()
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("FFmpeg transcoding failed: {}", stderr);
    }

    Ok(())
}

/// Encodes raw 16-bit 44.1kHz stereo PCM samples directly to the requested format using FFmpeg.
pub fn encode_pcm_to_file(
    pcm_samples: &[i16],
    sample_rate: u32,
    channels: u16,
    output_file: &Path,
    format: AudioFormat,
) -> Result<()> {
    let ffmpeg_bin = find_ffmpeg().unwrap_or_else(|| PathBuf::from("ffmpeg"));

    let mut cmd = silent_command(&ffmpeg_bin);
    cmd.arg("-y");
    cmd.args(["-f", "s16le"]);
    cmd.args(["-ar", &sample_rate.to_string()]);
    cmd.args(["-ac", &channels.to_string()]);
    cmd.args(["-i", "pipe:0"]);

    match format {
        AudioFormat::Mp3 => {
            cmd.args(["-c:a", "libmp3lame", "-b:a", "320k"]);
        }
        AudioFormat::M4a => {
            cmd.args(["-c:a", "alac"]);
        }
        AudioFormat::Flac => {
            cmd.args(["-c:a", "flac"]);
        }
    }

    cmd.arg(output_file);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().with_context(|| {
        format!(
            "Failed to spawn FFmpeg process ({}) for PCM encoding",
            ffmpeg_bin.display()
        )
    })?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let mut byte_buffer = Vec::with_capacity(pcm_samples.len() * 2);
        for sample in pcm_samples {
            byte_buffer.extend_from_slice(&sample.to_le_bytes());
        }
        let _ = stdin.write_all(&byte_buffer);
        let _ = stdin.flush();
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("FFmpeg encoding failed: {}", stderr);
    }

    Ok(())
}
