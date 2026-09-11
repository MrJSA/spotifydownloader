use egui::{ComboBox, CornerRadius, RichText, Stroke};
use crate::downloader::encoder::find_ffmpeg;
use crate::model::{AudioFormat, UserSettings};
use crate::theme::{Palette, RADIUS};

pub fn render_settings_view(
    ui: &mut egui::Ui,
    settings: &mut UserSettings,
    palette: &Palette,
) {
    ui.add_space(8.0);
    ui.heading(RichText::new("Settings").color(palette.text));
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Section: Download Storage & Folder Structure
        egui::Frame::NONE
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(CornerRadius::same(RADIUS as u8))
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.label(RichText::new("📂 Download Directory & Organization").size(15.0).color(palette.text).strong());
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Songs are automatically sorted into hierarchical folders: Artist -> Album -> songs.")
                        .color(palette.dim),
                );
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Download Root:").color(palette.secondary));
                    ui.monospace(settings.download_dir.display().to_string());

                    if ui.button("Browse...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            settings.download_dir = folder;
                        }
                    }
                });

                ui.add_space(6.0);
                ui.label(
                    RichText::new("Folder Structure Preview: ")
                        .size(12.0)
                        .color(palette.dim),
                );
                ui.monospace(format!(
                    "{}/Ed Sheeran/÷ (Divide)/04 - Shape of You.{}",
                    settings.download_dir.display(),
                    settings.default_format.extension()
                ));
            });

        ui.add_space(16.0);

        // Section: Audio Format & Quality
        egui::Frame::NONE
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(CornerRadius::same(RADIUS as u8))
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.label(RichText::new("🎵 Audio Format & Quality").size(15.0).color(palette.text).strong());
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Default Download Format:").color(palette.secondary));

                    ComboBox::from_id_salt("default_audio_format")
                        .selected_text(settings.default_format.display_name())
                        .show_ui(ui, |ui| {
                            for fmt in &AudioFormat::ALL {
                                ui.selectable_value(&mut settings.default_format, *fmt, fmt.display_name());
                            }
                        });
                });

                ui.add_space(8.0);
                ui.label(
                    RichText::new("• FLAC: True Lossless 16/24-bit audio stream (Best fidelity)\n• M4A: ALAC Apple Lossless audio stream (Native Apple/iTunes Lossless)\n• MP3: 320 kbps high-bitrate LAME encoding")
                        .size(11.0)
                        .color(palette.dim),
                );
            });

        ui.add_space(16.0);

        // Section: Live Play & Record Mode
        egui::Frame::NONE
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(CornerRadius::same(RADIUS as u8))
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.label(RichText::new("🎙 Live Playback & Simultaneous Recording").size(15.0).color(palette.text).strong());
                ui.add_space(8.0);

                ui.checkbox(
                    &mut settings.play_while_recording,
                    RichText::new("Play audio to speakers simultaneously while recording").color(palette.text),
                );
                ui.label(
                    RichText::new("When enabled, audio is routed to your default output device in real-time while uncompressed samples are recorded.")
                        .size(11.0)
                        .color(palette.dim),
                );
            });

        ui.add_space(16.0);

        // Section: System Backend & Tools Detection
        egui::Frame::NONE
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(CornerRadius::same(RADIUS as u8))
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.label(RichText::new("🔧 Audio Engine & Tool Status").size(15.0).color(palette.text).strong());
                ui.add_space(8.0);

                let ffmpeg_status = find_ffmpeg();
                ui.horizontal(|ui| {
                    ui.label("FFmpeg Encoder:");
                    if let Some(path) = ffmpeg_status {
                        crate::theme::paint_checkmark(ui, palette.accent, 14.0);
                        ui.label(RichText::new(format!("Found ({})", path.display())).color(palette.accent));
                    } else {
                        ui.label(RichText::new("⚠ Not found on PATH (Using bundled/fallback resolver)").color(palette.warning));
                    }
                });

                ui.add_space(4.0);
                ui.label(
                    RichText::new("FFmpeg is used for lossless FLAC encoding, MP3 320k, and M4A AAC conversion.")
                        .size(11.0)
                        .color(palette.dim),
                );
            });
    });
}
