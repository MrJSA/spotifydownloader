use egui::{Button, CornerRadius, RichText, Stroke, TextEdit};
use crate::model::AudioFormat;
use crate::theme::{Palette, RADIUS_SMALL, TOPBAR_HEIGHT};

pub fn render_topbar(
    ui: &mut egui::Ui,
    url_input: &mut String,
    current_format: &mut AudioFormat,
    is_fetching: bool,
    user_name: Option<&str>,
    palette: &Palette,
) -> (bool, bool) {
    let mut trigger_fetch = false;
    let mut trigger_account_click = false;

    ui.horizontal(|ui| {
        ui.set_height(TOPBAR_HEIGHT);
        ui.spacing_mut().item_spacing.x = 8.0;

        // 1. Left side: Search input, Paste, Search / Fetch
        let total_avail = ui.available_width();
        let search_box_width = (total_avail - 420.0).clamp(180.0, 360.0);

        let text_box = TextEdit::singleline(url_input)
            .hint_text("Search songs, artists, albums, or paste a Spotify link...")
            .desired_width(search_box_width)
            .margin(egui::Margin::symmetric(10, 8));

        let res = ui.add(text_box);
        if res.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            trigger_fetch = true;
        }

        let paste_btn = Button::new(RichText::new("Paste").size(12.0).color(palette.text))
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(CornerRadius::same(RADIUS_SMALL as u8));

        if ui.add(paste_btn).clicked() {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                if let Ok(text) = clipboard.get_text() {
                    if !text.is_empty() {
                        *url_input = text;
                    }
                }
            }
        }

        let fetch_btn_text = if is_fetching { "Searching..." } else { "Search / Fetch" };
        let fetch_btn = Button::new(RichText::new(fetch_btn_text).color(palette.on_accent).strong())
            .fill(palette.accent)
            .corner_radius(CornerRadius::same(RADIUS_SMALL as u8));

        if ui.add_enabled(!is_fetching && !url_input.trim().is_empty(), fetch_btn).clicked() {
            trigger_fetch = true;
        }

        // 2. Right side: Format Switcher & Account Button
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(4.0);

            // User Account Pill Button
            let account_text = match user_name {
                Some(name) => format!("🟢 {}", name),
                None => "👤 Sign In".to_string(),
            };

            let account_btn = Button::new(RichText::new(account_text).size(12.0).color(palette.text))
                .fill(palette.surface)
                .stroke(Stroke::new(1.0, palette.outline))
                .corner_radius(CornerRadius::same(12));

            if ui.add(account_btn).clicked() {
                trigger_account_click = true;
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Format Switcher: MP3, M4A, FLAC (rendered in reverse so left-to-right reads: FLAC, M4A, MP3)
            for fmt in [AudioFormat::Mp3, AudioFormat::M4a, AudioFormat::Flac] {
                let is_active = *current_format == fmt;
                let (fill_col, text_col, stroke) = if is_active {
                    match fmt {
                        AudioFormat::Flac => (palette.flac_badge, palette.text, Stroke::NONE),
                        AudioFormat::Mp3 => (palette.mp3_badge, palette.text, Stroke::NONE),
                        AudioFormat::M4a => (palette.m4a_badge, palette.on_accent, Stroke::NONE),
                    }
                } else {
                    (palette.surface, palette.secondary, Stroke::new(1.0, palette.outline))
                };

                let btn = Button::new(RichText::new(fmt.short_name()).size(12.0).color(text_col).strong())
                    .fill(fill_col)
                    .stroke(stroke)
                    .corner_radius(CornerRadius::same(RADIUS_SMALL as u8));

                if ui.add(btn).clicked() {
                    *current_format = fmt;
                }
            }

            ui.label(RichText::new("Format:").size(12.0).color(palette.dim));
        });
    });

    (trigger_fetch, trigger_account_click)
}
