use std::io::Read;
use std::path::Path;
use anyhow::{anyhow, Context, Result};
use librespot_core::session::Session;
use librespot_core::spotify_uri::SpotifyUri;
use librespot_metadata::{Metadata, Track};
use librespot_audio::AudioFile;

/// Downloads a track directly from Spotify's CDN at 320 kbps (or highest available).
pub async fn download_spotify_track(
    session: &Session,
    track_id: &str,
    output_path: &Path,
) -> Result<()> {
    // 1. Resolve Spotify URI
    let clean_id = track_id.trim_start_matches("spotify:track:").trim();
    let uri_str = if clean_id.starts_with("spotify:") {
        clean_id.to_string()
    } else {
        format!("spotify:track:{}", clean_id)
    };

    let spotify_uri = SpotifyUri::from_uri(&uri_str)
        .map_err(|e| anyhow!("Invalid Spotify track ID '{}': {:?}", clean_id, e))?;

    // 2. Fetch track metadata from Spotify
    let track = Track::get(session, &spotify_uri).await
        .map_err(|e| anyhow!("Failed to fetch track metadata from Spotify: {:?}", e))?;

    // 3. Find the highest bitrate audio file (320 kbps Vorbis preferred)
    let file_id = track.files.get(&librespot_metadata::audio::AudioFileFormat::OGG_VORBIS_320)
        .or_else(|| track.files.get(&librespot_metadata::audio::AudioFileFormat::OGG_VORBIS_160))
        .or_else(|| track.files.get(&librespot_metadata::audio::AudioFileFormat::OGG_VORBIS_96))
        .or_else(|| track.files.iter().map(|(_, id)| id).next())
        .copied()
        .ok_or_else(|| anyhow!("No playable audio stream found for track in Spotify catalog"))?;

    // 4. Open audio file with keymaster decryption
    let mut audio_file = AudioFile::open(session, file_id, 1024 * 1024).await
        .map_err(|e| anyhow!("Spotify Keymaster denied audio key: {:?}", e))?;

    // 5. Stream decrypted audio bytes into the destination file
    let mut buffer = Vec::with_capacity(1024 * 1024 * 4); // 4MB initial buffer
    audio_file.read_to_end(&mut buffer)
        .context("Failed while reading decrypted audio stream from Spotify CDN")?;

    if buffer.is_empty() {
        return Err(anyhow!("Received empty audio stream from Spotify CDN"));
    }

    std::fs::write(output_path, buffer)
        .context("Failed to write downloaded Spotify stream to disk")?;

    Ok(())
}
