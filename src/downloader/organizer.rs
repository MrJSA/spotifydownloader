use std::path::{Path, PathBuf};
use crate::model::{AudioFormat, TrackMetadata};

/// Sanitizes a string so it can be safely used as a filename or folder name across OSes.
pub fn sanitize_filename(input: &str) -> String {
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let cleaned: String = input
        .chars()
        .map(|c| if invalid_chars.contains(&c) || c.is_control() { '_' } else { c })
        .collect();

    let trimmed = cleaned.trim_matches(&[' ', '.'] as &[char]).to_string();
    if trimmed.is_empty() {
        "Unknown".to_string()
    } else {
        trimmed
    }
}

/// Computes the structured destination path:
/// `{download_dir}/{Artist}/{Album}/{TrackNumber} - {Title}.{ext}`
pub fn get_destination_path(
    download_dir: &Path,
    track: &TrackMetadata,
    format: AudioFormat,
) -> PathBuf {
    let artist_folder = sanitize_filename(track.primary_artist());
    let album_folder = sanitize_filename(&track.album);

    let track_num_str = if track.track_number > 0 {
        format!("{:02} - ", track.track_number)
    } else {
        String::new()
    };

    let filename = format!(
        "{}{}.{}",
        track_num_str,
        sanitize_filename(&track.title),
        format.extension()
    );

    download_dir
        .join(artist_folder)
        .join(album_folder)
        .join(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitization() {
        assert_eq!(sanitize_filename("AC/DC"), "AC_DC");
        assert_eq!(sanitize_filename("What? \"Quote\""), "What_ _Quote_");
        assert_eq!(sanitize_filename("Song: The Sequel*"), "Song_ The Sequel_");
    }

    #[test]
    fn test_destination_path() {
        let track = TrackMetadata {
            id: "123".into(),
            title: "Shape of You".into(),
            artists: vec!["Ed Sheeran".into()],
            album_artist: Some("Ed Sheeran".into()),
            album: "÷ (Divide)".into(),
            release_date: "2017".into(),
            track_number: 4,
            disc_number: 1,
            duration_ms: 233000,
            isrc: None,
            genre: Some("Pop".into()),
            cover_url: None,
            spotify_url: "".into(),
        };

        let path = get_destination_path(Path::new("C:/Music"), &track, AudioFormat::Flac);
        assert_eq!(
            path,
            PathBuf::from("C:/Music/Ed Sheeran/÷ (Divide)/04 - Shape of You.flac")
        );
    }
}
