pub mod encoder;
pub mod organizer;
pub mod recorder;
pub mod spotify_stream;
pub mod tagger;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;
use anyhow::{Context, Result, bail};
use tokio::sync::mpsc;

use crate::auth::AuthManager;
use crate::downloader::encoder::silent_command;
use crate::model::{AudioFormat, DownloadItem, DownloadStatus, TrackMetadata, UserSettings};
use crate::spotify::SpotifyClient;

/// Event sent from the downloader workers to update the UI.
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    StatusChanged {
        id: String,
        status: DownloadStatus,
        destination_path: Option<PathBuf>,
    },
    ItemFinished {
        id: String,
        success: bool,
    },
}

pub struct DownloadManager {
    pub queue: Arc<Mutex<Vec<DownloadItem>>>,
    pub settings: Arc<Mutex<UserSettings>>,
    pub auth: Arc<AuthManager>,
    spotify: Arc<SpotifyClient>,
    sender: mpsc::UnboundedSender<DownloadEvent>,
}

impl DownloadManager {
    pub fn new(
        settings: Arc<Mutex<UserSettings>>,
        auth: Arc<AuthManager>,
        spotify: Arc<SpotifyClient>,
        sender: mpsc::UnboundedSender<DownloadEvent>,
    ) -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
            settings,
            auth,
            spotify,
            sender,
        }
    }

    /// Enqueues a track for download.
    pub fn enqueue(&self, track: TrackMetadata, format: AudioFormat) -> String {
        let id = format!("{}_{}", track.id, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
        let item = DownloadItem {
            id: id.clone(),
            track,
            format,
            status: DownloadStatus::Queued,
            destination_path: None,
        };

        {
            let mut q = self.queue.lock().unwrap();
            q.push(item.clone());
        }

        self.spawn_download_task(item);
        id
    }

    /// Retries a failed or completed download.
    pub fn retry(&self, id: &str) {
        let mut item_opt = None;
        if let Ok(mut q) = self.queue.lock() {
            if let Some(item) = q.iter_mut().find(|i| i.id == id) {
                item.status = DownloadStatus::Queued;
                item_opt = Some(item.clone());
            }
        }

        if let Some(item) = item_opt {
            let _ = self.sender.send(DownloadEvent::StatusChanged {
                id: item.id.clone(),
                status: DownloadStatus::Queued,
                destination_path: None,
            });
            self.spawn_download_task(item);
        }
    }

    /// Removes an item from the queue.
    pub fn remove(&self, id: &str) {
        if let Ok(mut q) = self.queue.lock() {
            q.retain(|i| i.id != id);
        }
    }

    /// Clears all finished (completed or failed) items from the queue.
    pub fn clear_finished(&self) {
        if let Ok(mut q) = self.queue.lock() {
            q.retain(|i| !i.status.is_finished());
        }
    }

    fn spawn_download_task(&self, item: DownloadItem) {
        let sender = self.sender.clone();
        let settings_arc = self.settings.clone();
        let spotify_arc = self.spotify.clone();
        let auth_arc = self.auth.clone();

        tokio::spawn(async move {
            let id = item.id.clone();
            let track = item.track.clone();
            let format = item.format;

            let result = run_download_pipeline(
                &id,
                &track,
                format,
                &settings_arc,
                &spotify_arc,
                &auth_arc,
                &sender,
            ).await;

            match result {
                Ok(dest_path) => {
                    let _ = sender.send(DownloadEvent::StatusChanged {
                        id: id.clone(),
                        status: DownloadStatus::Completed { file_path: dest_path.clone() },
                        destination_path: Some(dest_path),
                    });
                    let _ = sender.send(DownloadEvent::ItemFinished { id, success: true });
                }
                Err(err) => {
                    let _ = sender.send(DownloadEvent::StatusChanged {
                        id: id.clone(),
                        status: DownloadStatus::Failed { error: err.to_string() },
                        destination_path: None,
                    });
                    let _ = sender.send(DownloadEvent::ItemFinished { id, success: false });
                }
            }
        });
    }
}

async fn run_download_pipeline(
    id: &str,
    track: &TrackMetadata,
    format: AudioFormat,
    settings_arc: &Arc<Mutex<UserSettings>>,
    spotify: &SpotifyClient,
    auth: &Arc<AuthManager>,
    sender: &mpsc::UnboundedSender<DownloadEvent>,
) -> Result<PathBuf> {
    // 1. Update status to Resolving
    let _ = sender.send(DownloadEvent::StatusChanged {
        id: id.to_string(),
        status: DownloadStatus::Resolving,
        destination_path: None,
    });

    let (download_dir, _play_while_recording) = {
        let s = settings_arc.lock().unwrap();
        (s.download_dir.clone(), s.play_while_recording)
    };

    // Determine final destination path (Artist -> Album -> songs)
    let dest_path = organizer::get_destination_path(&download_dir, track, format);
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create destination directories")?;

        // Clean up any stray .opus files or temp files previously left in this folder
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("opus") || path.to_string_lossy().contains("tmp.download") {
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }
    }

    // 2. Fetch cover art if available
    let cover_data = if let Some(url) = &track.cover_url {
        spotify.fetch_cover_art(url).await.ok()
    } else {
        None
    };

    // Also save cover.jpg into the album directory if available
    if let (Some(parent), Some(cover)) = (dest_path.parent(), &cover_data) {
        let cover_path = parent.join("cover.jpg");
        if !cover_path.exists() {
            let _ = std::fs::write(&cover_path, cover);
        }
    }

    // Create an isolated temporary directory for downloading audio chunks
    let temp_dir = std::env::temp_dir().join("spotifydownloader_temp").join(id);
    std::fs::create_dir_all(&temp_dir).context("Failed to create temporary download directory")?;

    // 3. Resolve & Stream audio into temp folder
    let temp_audio_file = match download_audio_stream(id, track, &temp_dir, auth, sender).await {
        Ok(path) => path,
        Err(err) => {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(err);
        }
    };

    // 4. Transcode to requested format (MP3 320k, M4A 256k, or FLAC)
    let _ = sender.send(DownloadEvent::StatusChanged {
        id: id.to_string(),
        status: DownloadStatus::Converting,
        destination_path: Some(dest_path.clone()),
    });

    let transcode_result = encoder::transcode_audio(&temp_audio_file, &dest_path, format)
        .context("Audio encoding failed");

    // Clean up temporary download directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    transcode_result?;

    // 5. Write metadata & embed cover art
    let _ = sender.send(DownloadEvent::StatusChanged {
        id: id.to_string(),
        status: DownloadStatus::Tagging,
        destination_path: Some(dest_path.clone()),
    });

    let mut enriched_track = track.clone();
    if enriched_track.genre.is_none() {
        if let Some(genre) = spotify.fetch_genre_fallback(track.primary_artist(), &track.album).await {
            enriched_track.genre = Some(genre);
        } else if let Some(genre) = spotify.fetch_genre_fallback(track.primary_artist(), &track.title).await {
            enriched_track.genre = Some(genre);
        }
    }

    tagger::tag_file(&dest_path, format, &enriched_track, cover_data.as_deref())
        .context("Failed to embed tags and cover art")?;

    Ok(dest_path)
}

/// Resolves audio stream and writes to temp_dir, returning the path to the downloaded audio file.
async fn download_audio_stream(
    id: &str,
    track: &TrackMetadata,
    temp_dir: &std::path::Path,
    auth: &Arc<AuthManager>,
    sender: &mpsc::UnboundedSender<DownloadEvent>,
) -> Result<PathBuf> {
    // Strategy 0: Direct Spotify CDN Download (if authenticated with Premium)
    if let Some(session) = auth.get_session().await {
        let spotify_stream_path = temp_dir.join("spotify_stream.ogg");
        eprintln!(
            "[Direct Spotify] Attempting 320kbps CDN stream for '{} - {}' (Track ID: {})",
            track.primary_artist(),
            track.title,
            track.id
        );

        match spotify_stream::download_spotify_track(&session, &track.id, &spotify_stream_path).await {
            Ok(()) => {
                if spotify_stream_path.exists() && std::fs::metadata(&spotify_stream_path).map(|m| m.len() > 0).unwrap_or(false) {
                    eprintln!("[Direct Spotify] Successfully downloaded track directly from Spotify servers at 320 kbps!");
                    return Ok(spotify_stream_path);
                }
            }
            Err(e) => {
                eprintln!("[Direct Spotify] Stream attempt failed: {:?}; falling back to multi-source resolver.", e);
            }
        }
    } else {
        eprintln!(
            "[Downloader] Direct Spotify CDN session not connected (requires Option B credentials). Using multi-source resolver."
        );
    }

    let output_template = temp_dir.join("stream.%(ext)s");

    // Strategy A: Check if yt-dlp is available
    let yt_dlp_path = find_ytdlp();
    let query = format!("ytsearch1:{} - {}", track.primary_artist(), track.title);

    if let Some(ytdlp) = &yt_dlp_path {
        let mut cmd = silent_command(ytdlp);
        cmd.arg("-f").arg("bestaudio/best");
        cmd.arg("--no-playlist");
        cmd.arg("-x");
        cmd.arg("--output").arg(&output_template);
        cmd.arg(&query);

        if let Ok(out) = cmd.output() {
            if out.status.success() {
                if let Some(file) = find_audio_file_in_dir(temp_dir) {
                    return Ok(file);
                }
            }
        }
    }

    // Strategy B: Query SongLink / Odesli for YouTube link
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/124.0.0.0")
        .build()
        .unwrap_or_default();

    let songlink_url = format!(
        "https://api.song.link/v1-alpha.1/links?url={}&userCountry=US",
        urlencoding::encode(&track.spotify_url)
    );

    let mut stream_url: Option<String> = None;
    if let Ok(resp) = client.get(&songlink_url).send().await {
        if let Ok(val) = resp.json::<serde_json::Value>().await {
            if let Some(links) = val.get("linksByPlatform") {
                if let Some(yt_link) = links.get("youtube").and_then(|y| y["url"].as_str()) {
                    stream_url = Some(yt_link.to_string());
                }
            }
        }
    }

    // If we have yt-dlp and direct link, try with that
    if let (Some(ytdlp), Some(url)) = (&yt_dlp_path, &stream_url) {
        let mut cmd = silent_command(ytdlp);
        cmd.arg("-f").arg("bestaudio/best");
        cmd.arg("--no-playlist");
        cmd.arg("-x");
        cmd.arg("--output").arg(&output_template);
        cmd.arg(url);

        if let Ok(out) = cmd.output() {
            if out.status.success() {
                if let Some(file) = find_audio_file_in_dir(temp_dir) {
                    return Ok(file);
                }
            }
        }
    }

    // Strategy C: High quality stream fetch via Cobalt endpoints
    let cobalt_endpoints = [
        "https://api.cobalt.tools/api/json",
        "https://co.wuk.sh/api/json",
    ];

    let target_video_url = stream_url.unwrap_or_else(|| {
        format!("https://music.youtube.com/search?q={}", urlencoding::encode(&format!("{} {}", track.primary_artist(), track.title)))
    });

    let raw_temp_file = temp_dir.join("stream.raw");

    for endpoint in cobalt_endpoints {
        let payload = serde_json::json!({
            "url": target_video_url,
            "isAudioOnly": true,
            "aFormat": "mp3",
        });

        if let Ok(resp) = client.post(endpoint)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
        {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(audio_direct_url) = json["url"].as_str() {
                    if let Ok(mut download_resp) = client.get(audio_direct_url).send().await {
                        let total_size = download_resp.content_length().unwrap_or(1);
                        use tokio::io::AsyncWriteExt;
                        let mut file = tokio::fs::File::create(&raw_temp_file).await?;
                        let mut downloaded: u64 = 0;
                        let start = Instant::now();

                        while let Ok(Some(chunk)) = download_resp.chunk().await {
                            file.write_all(&chunk).await?;
                            downloaded += chunk.len() as u64;

                            let elapsed = start.elapsed().as_secs_f32().max(0.1);
                            let speed_kbps = (downloaded as f32 / 1024.0) / elapsed;
                            let percent = (downloaded as f32 / total_size as f32).min(1.0);

                            let _ = sender.send(DownloadEvent::StatusChanged {
                                id: id.to_string(),
                                status: DownloadStatus::Downloading { percent, speed_kbps },
                                destination_path: None,
                            });
                        }
                        file.flush().await?;
                        if raw_temp_file.exists() {
                            return Ok(raw_temp_file);
                        }
                    }
                }
            }
        }
    }

    // Fallback: yt-dlp search query
    if let Some(ytdlp) = &yt_dlp_path {
        let mut cmd = silent_command(ytdlp);
        cmd.arg("-f").arg("bestaudio/best");
        cmd.arg("--no-playlist");
        cmd.arg("-x");
        cmd.arg("--output").arg(&output_template);
        cmd.arg(format!("ytsearch:{} {}", track.primary_artist(), track.title));

        if let Ok(out) = cmd.output() {
            if out.status.success() {
                if let Some(file) = find_audio_file_in_dir(temp_dir) {
                    return Ok(file);
                }
            }
        }
    }

    bail!("Could not resolve audio stream for track '{}'. Ensure an internet connection or yt-dlp/ffmpeg are available.", track.title)
}

fn find_audio_file_in_dir(dir: &std::path::Path) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if ["opus", "webm", "m4a", "mp3", "ogg", "flac", "wav", "aac", "raw"].contains(&ext_lower.as_str()) {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

static CACHED_YTDLP: OnceLock<Option<PathBuf>> = OnceLock::new();

fn find_ytdlp() -> Option<PathBuf> {
    CACHED_YTDLP.get_or_init(|| {
        let mut cmd = silent_command("yt-dlp");
        cmd.arg("--version");
        if let Ok(out) = cmd.output() {
            if out.status.success() {
                return Some(PathBuf::from("yt-dlp"));
            }
        }

        let reference = PathBuf::from(r"c:\Users\joshu\RustroverProjects\spotifydownloader\.private\Spotify-Downloader-main\Spotify Downloader\yt-dlp.exe");
        if reference.exists() {
            return Some(reference);
        }

        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let local_ytdlp = dir.join("yt-dlp.exe");
                if local_ytdlp.exists() {
                    return Some(local_ytdlp);
                }
            }
        }

        None
    }).clone()
}
