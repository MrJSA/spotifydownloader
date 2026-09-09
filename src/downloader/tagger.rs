use std::path::Path;
use anyhow::{Context, Result};
use id3::TagLike;
use crate::model::{AudioFormat, TrackMetadata};

/// Embeds rich metadata and album artwork into an audio file according to its format.
pub fn tag_file(
    file_path: &Path,
    format: AudioFormat,
    metadata: &TrackMetadata,
    cover_data: Option<&[u8]>,
) -> Result<()> {
    match format {
        AudioFormat::Mp3 => tag_mp3(file_path, metadata, cover_data),
        AudioFormat::Flac => tag_flac(file_path, metadata, cover_data),
        AudioFormat::M4a => tag_m4a(file_path, metadata, cover_data),
    }
}

fn tag_mp3(file_path: &Path, metadata: &TrackMetadata, cover_data: Option<&[u8]>) -> Result<()> {
    let mut tag = id3::Tag::read_from_path(file_path).unwrap_or_default();

    tag.set_title(&metadata.title);
    tag.set_artist(metadata.artists_string());
    if let Some(album_artist) = &metadata.album_artist {
        tag.set_album_artist(album_artist);
    }
    tag.set_album(&metadata.album);
    tag.set_track(metadata.track_number);
    tag.set_disc(metadata.disc_number);

    if let Ok(year) = metadata.release_date.split('-').next().unwrap_or("").parse::<i32>() {
        tag.set_year(year);
    }

    if let Some(cover) = cover_data {
        let mime = detect_image_mime(cover);
        tag.add_frame(id3::frame::Picture {
            mime_type: mime.to_string(),
            picture_type: id3::frame::PictureType::CoverFront,
            description: "Cover".to_string(),
            data: cover.to_vec(),
        });
    }

    tag.write_to_path(file_path, id3::Version::Id3v24)
        .context("Failed to write ID3v2 tags to MP3")?;
    Ok(())
}

fn tag_flac(file_path: &Path, metadata: &TrackMetadata, cover_data: Option<&[u8]>) -> Result<()> {
    let mut tag = metaflac::Tag::read_from_path(file_path).unwrap_or_default();

    let vorbis = tag.vorbis_comments_mut();
    vorbis.set_title(vec![metadata.title.clone()]);
    vorbis.set_artist(vec![metadata.artists_string()]);
    if let Some(album_artist) = &metadata.album_artist {
        vorbis.set("ALBUMARTIST", vec![album_artist.clone()]);
    }
    vorbis.set_album(vec![metadata.album.clone()]);
    vorbis.set_track(metadata.track_number);

    if !metadata.release_date.is_empty() {
        vorbis.set("DATE", vec![metadata.release_date.clone()]);
    }

    if let Some(isrc) = &metadata.isrc {
        vorbis.set("ISRC", vec![isrc.clone()]);
    }

    if let Some(cover) = cover_data {
        let mime = detect_image_mime(cover);
        tag.remove_picture_type(metaflac::block::PictureType::CoverFront);
        tag.add_picture(
            mime,
            metaflac::block::PictureType::CoverFront,
            cover.to_vec(),
        );
    }

    tag.write_to_path(file_path)
        .context("Failed to write Vorbis tags to FLAC")?;
    Ok(())
}

fn tag_m4a(file_path: &Path, metadata: &TrackMetadata, cover_data: Option<&[u8]>) -> Result<()> {
    let mut tag = mp4ameta::Tag::read_from_path(file_path).unwrap_or_default();

    tag.set_title(&metadata.title);
    tag.set_artist(metadata.artists_string());
    if let Some(album_artist) = &metadata.album_artist {
        tag.set_album_artist(album_artist);
    }
    tag.set_album(&metadata.album);
    tag.set_track_number(metadata.track_number as u16);
    tag.set_disc_number(metadata.disc_number as u16);

    if !metadata.release_date.is_empty() {
        tag.set_year(&metadata.release_date);
    }

    if let Some(cover) = cover_data {
        let img = if is_png(cover) {
            mp4ameta::Img::png(cover.to_vec())
        } else {
            mp4ameta::Img::jpeg(cover.to_vec())
        };
        tag.add_artwork(img);
    }

    tag.write_to_path(file_path)
        .context("Failed to write MP4 tags to M4A")?;
    Ok(())
}

fn detect_image_mime(data: &[u8]) -> &'static str {
    if is_png(data) {
        "image/png"
    } else {
        "image/jpeg"
    }
}

fn is_png(data: &[u8]) -> bool {
    data.len() >= 8 && &data[0..8] == b"\x89PNG\r\n\x1a\n"
}
