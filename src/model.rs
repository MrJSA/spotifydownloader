use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported output audio formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    #[serde(alias = "Mp3")]
    Mp3_320,
    Mp3_192,
    Mp3_128,
    Flac,
    #[serde(alias = "M4a")]
    M4aAlac,
    M4aAac,
    Wav,
    Aiff,
}

impl AudioFormat {
    pub const ALL: [Self; 8] = [
        Self::Flac,
        Self::M4aAlac,
        Self::Wav,
        Self::Aiff,
        Self::M4aAac,
        Self::Mp3_320,
        Self::Mp3_192,
        Self::Mp3_128,
    ];

    // Quick-switch presets for the top bar
    pub const QUICK_PRESETS: [Self; 4] = [
        Self::Flac,
        Self::M4aAlac,
        Self::M4aAac,
        Self::Mp3_320,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Flac => "FLAC (Lossless)",
            Self::M4aAlac => "M4A (ALAC Lossless)",
            Self::Wav => "WAV (Lossless 16-bit)",
            Self::Aiff => "AIFF (Lossless 16-bit)",
            Self::M4aAac => "M4A (256 kbps AAC)",
            Self::Mp3_320 => "MP3 (320 kbps)",
            Self::Mp3_192 => "MP3 (192 kbps)",
            Self::Mp3_128 => "MP3 (128 kbps)",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Flac => "FLAC",
            Self::M4aAlac => "ALAC",
            Self::Wav => "WAV",
            Self::Aiff => "AIFF",
            Self::M4aAac => "AAC",
            Self::Mp3_320 => "MP3 320k",
            Self::Mp3_192 => "MP3 192k",
            Self::Mp3_128 => "MP3 128k",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Flac => "flac",
            Self::M4aAlac | Self::M4aAac => "m4a",
            Self::Wav => "wav",
            Self::Aiff => "aiff",
            Self::Mp3_320 | Self::Mp3_192 | Self::Mp3_128 => "mp3",
        }
    }

    pub fn is_lossless(&self) -> bool {
        matches!(self, Self::Flac | Self::M4aAlac | Self::Wav | Self::Aiff)
    }
}

/// Metadata extracted for a Spotify track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub id: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album_artist: Option<String>,
    pub album: String,
    pub release_date: String,
    pub track_number: u32,
    pub disc_number: u32,
    pub duration_ms: u64,
    pub isrc: Option<String>,
    pub genre: Option<String>,
    pub cover_url: Option<String>,
    pub spotify_url: String,
}

impl TrackMetadata {
    pub fn artists_string(&self) -> String {
        if self.artists.is_empty() {
            "Unknown Artist".to_string()
        } else {
            self.artists.join(", ")
        }
    }

    pub fn primary_artist(&self) -> &str {
        if let Some(album_artist) = &self.album_artist {
            if !album_artist.is_empty() {
                return album_artist.as_str();
            }
        }

        if let Some(first) = self.artists.first() {
            // If the artist string contains multiple artists separated by comma, take the first one
            let main = first.split(',').next().unwrap_or(first).trim();
            if !main.is_empty() {
                return main;
            }
        }
        "Unknown Artist"
    }

    pub fn duration_formatted(&self) -> String {
        let total_secs = self.duration_ms / 1000;
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{}:{:02}", mins, secs)
    }
}

/// A collection of tracks fetched from an album, playlist, or single track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchedCollection {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub cover_url: Option<String>,
    pub tracks: Vec<TrackMetadata>,
    pub collection_type: CollectionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollectionType {
    Track,
    Album,
    Playlist,
}

/// An album in Spotify search results or artist discography.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogAlbum {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub release_date: String,
    pub total_tracks: u32,
    pub cover_url: Option<String>,
}

/// An artist in Spotify search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogArtist {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
}

/// Catalog search results combining albums, artists, and tracks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CatalogSearchResults {
    pub query: String,
    pub albums: Vec<CatalogAlbum>,
    pub artists: Vec<CatalogArtist>,
    pub tracks: Vec<TrackMetadata>,
}

/// Status of an active or queued download.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DownloadStatus {
    Queued,
    Resolving,
    Downloading { percent: f32, speed_kbps: f32 },
    Recording { seconds_recorded: f32 },
    Converting,
    Tagging,
    Completed { file_path: PathBuf },
    Failed { error: String },
}

impl DownloadStatus {
    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Completed { .. } | Self::Failed { .. })
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Resolving
                | Self::Downloading { .. }
                | Self::Recording { .. }
                | Self::Converting
                | Self::Tagging
        )
    }

    pub fn label(&self) -> String {
        match self {
            Self::Queued => "Queued".to_string(),
            Self::Resolving => "Resolving audio stream...".to_string(),
            Self::Downloading { percent, speed_kbps } => {
                if *speed_kbps > 1024.0 {
                    format!("{:.1}% ({:.2} MB/s)", percent * 100.0, speed_kbps / 1024.0)
                } else {
                    format!("{:.1}% ({:.0} KB/s)", percent * 100.0, speed_kbps)
                }
            }
            Self::Recording { seconds_recorded } => {
                format!("Recording audio ({:.1}s)", seconds_recorded)
            }
            Self::Converting => "Transcoding audio...".to_string(),
            Self::Tagging => "Writing metadata & cover...".to_string(),
            Self::Completed { .. } => "Completed".to_string(),
            Self::Failed { error } => format!("Failed: {}", error),
        }
    }
}

/// Represents an item in the download queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItem {
    pub id: String,
    pub track: TrackMetadata,
    pub format: AudioFormat,
    pub status: DownloadStatus,
    pub destination_path: Option<PathBuf>,
}

/// Application settings persisted to disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSettings {
    pub download_dir: PathBuf,
    pub default_format: AudioFormat,
    pub play_while_recording: bool,
    pub max_concurrent_downloads: usize,
    pub folder_structure: String,
}

impl Default for UserSettings {
    fn default() -> Self {
        let download_dir = directories::UserDirs::new()
            .and_then(|u| u.audio_dir().map(|p| p.to_path_buf()))
            .or_else(|| {
                directories::UserDirs::new()
                    .and_then(|u| u.download_dir().map(|p| p.join("Spotify Downloads")))
            })
            .unwrap_or_else(|| PathBuf::from("downloads"));

        Self {
            download_dir,
            default_format: AudioFormat::Flac,
            play_while_recording: false,
            max_concurrent_downloads: 3,
            folder_structure: "{artist}/{album}/{track_number} - {title}".to_string(),
        }
    }
}

impl UserSettings {
    pub fn config_path() -> PathBuf {
        let dir = directories::ProjectDirs::from("com", "spotifydownloader", "SpotifyDownloader")
            .map(|p| p.config_dir().to_path_buf())
            .or_else(|| directories::UserDirs::new().map(|u| u.home_dir().join(".spotifydownloader")))
            .unwrap_or_else(|| PathBuf::from("config"));
        let _ = std::fs::create_dir_all(&dir);
        dir.join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str::<UserSettings>(&content) {
                return settings;
            }
        }
        let default = Self::default();
        let _ = default.save();
        default
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_settings_serialization_round_trip() {
        let settings = UserSettings {
            download_dir: PathBuf::from("C:/Custom/Music/Path"),
            default_format: AudioFormat::M4aAlac,
            play_while_recording: true,
            max_concurrent_downloads: 5,
            folder_structure: "{artist}/{album}/{title}".to_string(),
        };

        let json = serde_json::to_string(&settings).expect("Must serialize");
        let deserialized: UserSettings = serde_json::from_str(&json).expect("Must deserialize");

        assert_eq!(settings, deserialized);
        assert_eq!(deserialized.default_format, AudioFormat::M4aAlac);

        // Test backwards compatibility deserializing legacy strings "M4a" and "Mp3"
        let legacy_json = r#"{"download_dir":"C:/Music","default_format":"M4a","play_while_recording":false,"max_concurrent_downloads":3,"folder_structure":""}"#;
        let legacy_settings: UserSettings = serde_json::from_str(legacy_json).expect("Legacy M4a must deserialize");
        assert_eq!(legacy_settings.default_format, AudioFormat::M4aAlac);

        let legacy_mp3_json = r#"{"download_dir":"C:/Music","default_format":"Mp3","play_while_recording":false,"max_concurrent_downloads":3,"folder_structure":""}"#;
        let legacy_mp3_settings: UserSettings = serde_json::from_str(legacy_mp3_json).expect("Legacy Mp3 must deserialize");
        assert_eq!(legacy_mp3_settings.default_format, AudioFormat::Mp3_320);

        assert_eq!(deserialized.download_dir, PathBuf::from("C:/Custom/Music/Path"));
    }
}
