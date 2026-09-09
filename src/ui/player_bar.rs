use egui::{CornerRadius, ProgressBar, RichText, Vec2};
use crate::downloader::DownloadManager;
use crate::model::{AudioFormat, DownloadStatus};
use crate::theme::{PLAYERBAR_HEIGHT, Palette, RADIUS_SMALL};

pub fn render_player_bar(
    ui: &mut egui::Ui,
    manager: &DownloadManager,
    palette: &Palette,
) {
    let queue = manager.queue.lock().unwrap().clone();
    let active_item = queue.iter().find(|i| i.status.is_active()).cloned();
    let completed_count = queue.iter().filter(|i| matches!(i.status, DownloadStatus::Completed { .. })).count();
    let total_count = queue.len();

    ui.horizontal(|ui| {
        ui.set_height(PLAYERBAR_HEIGHT);
        ui.add_space(12.0);

        // Left section: Active Track info
        if let Some(item) = &active_item {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, CornerRadius::same(RADIUS_SMALL as u8), palette.surface);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "♫",
                egui::FontId::proportional(20.0),
                palette.accent,
            );

            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&item.track.title).size(13.0).color(palette.text).strong());

                    let badge_color = match item.format {
                        AudioFormat::Flac => palette.flac_badge,
                        AudioFormat::Mp3 => palette.mp3_badge,
                        AudioFormat::M4a => palette.m4a_badge,
                    };
                    let badge_text_color = match item.format {
                        AudioFormat::Flac | AudioFormat::Mp3 => palette.text,
                        AudioFormat::M4a => palette.on_accent,
                    };

                    egui::Frame::NONE
                        .fill(badge_color)
                        .corner_radius(CornerRadius::same(3))
                        .inner_margin(egui::Margin::symmetric(4, 1))
                        .show(ui, |ui| {
                            ui.label(RichText::new(item.format.short_name()).size(9.0).color(badge_text_color).strong());
                        });
                });

                ui.label(RichText::new(item.track.primary_artist()).size(11.0).color(palette.dim));
            });
        } else {
            ui.label(RichText::new("Queue idle").color(palette.dim));
        }

        // Center section: Queue progress
        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
            if let Some(item) = &active_item {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(item.status.label()).size(11.0).color(palette.secondary));
                    if let DownloadStatus::Downloading { percent, .. } = item.status {
                        let pbar = ProgressBar::new(percent)
                            .desired_width(260.0)
                            .fill(palette.accent);
                        ui.add(pbar);
                    }
                });
            } else if total_count > 0 {
                ui.label(
                    RichText::new(format!("All downloads complete ({} / {} tracks)", completed_count, total_count))
                        .size(12.0)
                        .color(palette.accent),
                );
            }
        });

        // Right section: Quick actions
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(12.0);
            if ui.button("📂 Open Folder").clicked() {
                let s = manager.settings.lock().unwrap();
                let _ = open::that(&s.download_dir);
            }
        });
    });
}
