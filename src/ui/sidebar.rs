use egui::{Color32, CornerRadius, RichText, Sense, Vec2};
use crate::theme::{Palette, RADIUS_SMALL, SIDEBAR_WIDTH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationTab {
    Queue,
    Search,
    Account,
    Library,
    Settings,
}

pub fn render_sidebar(
    ui: &mut egui::Ui,
    current_tab: &mut NavigationTab,
    queue_count: usize,
    palette: &Palette,
) {
    ui.vertical(|ui| {
        ui.set_width(SIDEBAR_WIDTH);
        ui.add_space(16.0);

        // App Logo & Title
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            let (rect, _response) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
            ui.painter().circle_filled(rect.center(), 14.0, palette.accent);
            // Draw a musical note / download glyph
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "♫",
                egui::FontId::proportional(16.0),
                palette.on_accent,
            );

            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("SPOTIFY")
                        .size(11.0)
                        .color(palette.accent)
                        .strong(),
                );
                ui.label(
                    RichText::new("Downloader")
                        .size(15.0)
                        .color(palette.text)
                        .strong(),
                );
            });
        });

        ui.add_space(24.0);
        ui.separator();
        ui.add_space(12.0);

        // Navigation Items
        sidebar_button(
            ui,
            current_tab,
            NavigationTab::Queue,
            "📥  Download Queue",
            Some(queue_count),
            palette,
        );

        sidebar_button(
            ui,
            current_tab,
            NavigationTab::Search,
            "🔍  Search & Fetch",
            None,
            palette,
        );

        sidebar_button(
            ui,
            current_tab,
            NavigationTab::Account,
            "👤  Spotify Account",
            None,
            palette,
        );

        sidebar_button(
            ui,
            current_tab,
            NavigationTab::Library,
            "📂  Downloaded Library",
            None,
            palette,
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        sidebar_button(
            ui,
            current_tab,
            NavigationTab::Settings,
            "⚙  Settings",
            None,
            palette,
        );
    });
}

fn sidebar_button(
    ui: &mut egui::Ui,
    current_tab: &mut NavigationTab,
    tab: NavigationTab,
    label: &str,
    badge: Option<usize>,
    palette: &Palette,
) {
    let is_selected = *current_tab == tab;
    let bg_color = if is_selected {
        palette.surface_active
    } else {
        Color32::TRANSPARENT
    };
    let text_color = if is_selected {
        palette.text
    } else {
        palette.secondary
    };

    let desired_size = Vec2::new(ui.available_width() - 8.0, 36.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

    if response.hovered() && !is_selected {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(RADIUS_SMALL as u8),
            palette.surface_hover,
        );
    } else if bg_color != Color32::TRANSPARENT {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(RADIUS_SMALL as u8),
            bg_color,
        );
        // Green active indicator pill on left edge
        let pill_rect = egui::Rect::from_min_size(rect.left_center() - Vec2::new(0.0, 10.0), Vec2::new(3.0, 20.0));
        ui.painter().rect_filled(pill_rect, CornerRadius::same(2), palette.accent);
    }

    if response.clicked() {
        *current_tab = tab;
    }

    // Draw label text
    let text_pos = rect.left_center() + Vec2::new(12.0, 0.0);
    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        text_color,
    );

    // Draw badge count if present
    if let Some(count) = badge {
        if count > 0 {
            let badge_text = count.to_string();
            let badge_pos = rect.right_center() - Vec2::new(12.0, 0.0);
            let badge_rect = egui::Rect::from_center_size(badge_pos, Vec2::new(20.0, 18.0));
            ui.painter().rect_filled(badge_rect, CornerRadius::same(9), palette.accent);
            ui.painter().text(
                badge_pos,
                egui::Align2::CENTER_CENTER,
                badge_text,
                egui::FontId::proportional(11.0),
                palette.on_accent,
            );
        }
    }

    ui.add_space(2.0);
}
