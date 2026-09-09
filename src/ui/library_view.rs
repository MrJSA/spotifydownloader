use std::path::{Path, PathBuf};
use egui::{CornerRadius, RichText, Stroke};
use crate::model::UserSettings;
use crate::theme::{Palette, RADIUS, RADIUS_SMALL};

#[derive(Debug, Clone)]
pub struct LibraryTrack {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub format: String,
    pub size_mb: f32,
}

pub fn scan_library(dir: &Path) -> Vec<LibraryTrack> {
    let mut results = Vec::new();
    if !dir.exists() {
        return results;
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for artist_entry in entries.flatten() {
            if artist_entry.path().is_dir() {
                let artist_name = artist_entry.file_name().to_string_lossy().to_string();
                if let Ok(album_entries) = std::fs::read_dir(artist_entry.path()) {
                    for album_entry in album_entries.flatten() {
                        if album_entry.path().is_dir() {
                            let album_name = album_entry.file_name().to_string_lossy().to_string();
                            if let Ok(song_entries) = std::fs::read_dir(album_entry.path()) {
                                for song_entry in song_entries.flatten() {
                                    let path = song_entry.path();
                                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                        let ext_lower = ext.to_lowercase();
                                        if ["mp3", "m4a", "flac"].contains(&ext_lower.as_str()) {
                                            let size_mb = song_entry.metadata().map(|m| m.len() as f32 / (1024.0 * 1024.0)).unwrap_or(0.0);
                                            let file_stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                                            results.push(LibraryTrack {
                                                path,
                                                title: file_stem,
                                                artist: artist_name.clone(),
                                                album: album_name.clone(),
                                                format: ext_lower.to_uppercase(),
                                                size_mb,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    results
}

pub fn render_library_view(
    ui: &mut egui::Ui,
    settings: &UserSettings,
    palette: &Palette,
) {
    let tracks = scan_library(&settings.download_dir);

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.heading(RichText::new("Downloaded Library").color(palette.text));
        ui.add_space(8.0);
        ui.label(RichText::new(format!("({} songs)", tracks.len())).color(palette.dim));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("📂 Open Download Directory").clicked() {
                let _ = open::that(&settings.download_dir);
            }
        });
    });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);

    if tracks.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(RichText::new("📂").size(48.0));
            ui.add_space(8.0);
            ui.label(
                RichText::new("No downloaded songs found in your library")
                    .size(18.0)
                    .color(palette.text)
                    .strong(),
            );
            ui.label(
                RichText::new(format!("Current folder: {}", settings.download_dir.display()))
                    .color(palette.dim),
            );
        });
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for track in tracks {
            egui::Frame::NONE
                .fill(palette.surface)
                .stroke(Stroke::new(1.0, palette.outline))
                .corner_radius(CornerRadius::same(RADIUS as u8))
                .inner_margin(egui::Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Badge
                        let badge_color = match track.format.as_str() {
                            "FLAC" => palette.flac_badge,
                            "MP3" => palette.mp3_badge,
                            _ => palette.m4a_badge,
                        };

                        egui::Frame::NONE
                            .fill(badge_color)
                            .corner_radius(CornerRadius::same(RADIUS_SMALL as u8))
                            .inner_margin(egui::Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(&track.format)
                                        .size(11.0)
                                        .color(palette.text)
                                        .strong(),
                                );
                            });

                        ui.add_space(8.0);

                        ui.vertical(|ui| {
                            ui.label(RichText::new(&track.title).color(palette.text).strong());
                            ui.label(
                                RichText::new(format!("{} • {} • {:.1} MB", track.artist, track.album, track.size_mb))
                                    .size(12.0)
                                    .color(palette.dim),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let path_clone = track.path.clone();
                            if ui.button("📂 Open Folder").clicked() {
                                if let Some(parent) = path_clone.parent() {
                                    let _ = open::that(parent);
                                }
                            }
                            if ui.button("▶ Play").clicked() {
                                let _ = open::that(&path_clone);
                            }
                        });
                    });
                });
            ui.add_space(4.0);
        }
    });
}
