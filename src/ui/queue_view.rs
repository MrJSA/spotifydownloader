use egui::{CornerRadius, ProgressBar, RichText, Stroke};
use crate::downloader::DownloadManager;
use crate::model::{AudioFormat, DownloadStatus};
use crate::theme::{Palette, RADIUS, RADIUS_SMALL};

pub fn render_queue_view(
    ui: &mut egui::Ui,
    manager: &DownloadManager,
    palette: &Palette,
) {
    let queue_items = manager.queue.lock().unwrap().clone();

    ui.add_space(8.0);

    // Queue View Header
    ui.horizontal(|ui| {
        ui.heading(RichText::new("Download Queue").color(palette.text));
        ui.add_space(8.0);
        ui.label(RichText::new(format!("({} items)", queue_items.len())).color(palette.dim));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🗑 Clear Finished").clicked() {
                manager.clear_finished();
            }
        });
    });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);

    if queue_items.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(RichText::new("📥").size(48.0));
            ui.add_space(8.0);
            ui.label(
                RichText::new("No downloads in progress")
                    .size(18.0)
                    .color(palette.text)
                    .strong(),
            );
            ui.label(
                RichText::new("Paste a Spotify link in the top bar to fetch and download songs.")
                    .color(palette.dim),
            );
        });
        return;
    }

    let full_width = ui.available_width().max(400.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for item in queue_items {
                render_queue_row(ui, &item, manager, palette, full_width);
                ui.add_space(6.0);
            }
        });
}

fn render_queue_row(
    ui: &mut egui::Ui,
    item: &crate::model::DownloadItem,
    manager: &DownloadManager,
    palette: &Palette,
    row_width: f32,
) {
    let card_width = (row_width - 8.0).max(300.0);
    let inner_width = card_width - 24.0; // Subtract left & right 12px margin

    egui::Frame::NONE
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(CornerRadius::same(RADIUS as u8))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(inner_width);
            ui.set_max_width(inner_width);

            ui.horizontal(|ui| {
                // Format badge
                let badge_color = match item.format {
                    AudioFormat::Flac => palette.flac_badge,
                    AudioFormat::M4aAlac => palette.wav_badge,
                    AudioFormat::Wav => palette.wav_badge,
                    AudioFormat::Aiff => palette.aiff_badge,
                    AudioFormat::M4aAac => palette.m4a_badge,
                    AudioFormat::Mp3_320 | AudioFormat::Mp3_192 | AudioFormat::Mp3_128 => palette.mp3_badge,
                };
                let badge_frame = egui::Frame::NONE
                    .fill(badge_color)
                    .corner_radius(CornerRadius::same(RADIUS_SMALL as u8))
                    .inner_margin(egui::Margin::symmetric(6, 2));

                badge_frame.show(ui, |ui| {
                    ui.label(
                        RichText::new(item.format.short_name())
                            .size(11.0)
                            .color(color32_text_for_badge(item.format, palette))
                            .strong(),
                    );
                });

                ui.add_space(8.0);

                // Right controls width reserve: ~320px
                let right_controls_width = 330.0;
                let title_max_width = (inner_width - right_controls_width - 60.0).max(120.0);

                // Track Title & Artist (constrained to prevent pushing right buttons)
                ui.allocate_ui_with_layout(
                    egui::vec2(title_max_width, 36.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_max_width(title_max_width);
                        ui.add(
                            egui::Label::new(
                                RichText::new(&item.track.title)
                                    .size(14.0)
                                    .color(palette.text)
                                    .strong(),
                            )
                            .truncate(),
                        );
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("{} • {}", item.track.primary_artist(), item.track.album))
                                    .size(12.0)
                                    .color(palette.dim),
                            )
                            .truncate(),
                        );
                    },
                );

                // Status & Controls (pinned to the right)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Remove button
                    if ui.button("❌").clicked() {
                        manager.remove(&item.id);
                    }

                    match &item.status {
                        DownloadStatus::Completed { file_path } => {
                            let path_clone = file_path.clone();
                            if ui.button("📂 Open Folder").clicked() {
                                if let Some(parent) = path_clone.parent() {
                                    let _ = open::that(parent);
                                }
                            }
                            if ui.button("▶ Play").clicked() {
                                let _ = open::that(&path_clone);
                            }
                            ui.horizontal(|ui| {
                                crate::theme::paint_checkmark(ui, palette.accent, 14.0);
                                ui.label(
                                    RichText::new("Completed")
                                        .color(palette.accent)
                                        .strong(),
                                );
                            });
                        }
                        DownloadStatus::Failed { error } => {
                            let item_id = item.id.clone();
                            if ui.button("🔄 Retry").clicked() {
                                manager.retry(&item_id);
                            }
                            ui.label(
                                RichText::new(format!("⚠ {}", error))
                                    .color(palette.danger),
                            );
                        }
                        DownloadStatus::Downloading { percent, speed_kbps } => {
                            let speed_text = if *speed_kbps > 1024.0 {
                                format!("{:.1} MB/s", speed_kbps / 1024.0)
                            } else {
                                format!("{:.0} KB/s", speed_kbps)
                            };
                            ui.label(RichText::new(speed_text).size(11.0).color(palette.secondary));

                            let pbar = ProgressBar::new(*percent)
                                .desired_width(120.0)
                                .fill(palette.accent);
                            ui.add(pbar);
                        }
                        _ => {
                            ui.label(
                                RichText::new(item.status.label())
                                    .size(12.0)
                                    .color(palette.warning),
                            );
                        }
                    }
                });
            });
        });
}

fn color32_text_for_badge(format: AudioFormat, palette: &Palette) -> egui::Color32 {
    match format {
        AudioFormat::Flac | AudioFormat::Mp3_320 | AudioFormat::Mp3_192 | AudioFormat::Mp3_128 => palette.text,
        AudioFormat::M4aAlac | AudioFormat::M4aAac | AudioFormat::Wav | AudioFormat::Aiff => palette.on_accent,
    }
}
