use egui::{Color32, CornerRadius, Margin, Stroke, Vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub window: Color32,
    pub panel: Color32,
    pub surface: Color32,
    pub surface_hover: Color32,
    pub surface_active: Color32,
    pub outline: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub dim: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub warning: Color32,
    pub flac_badge: Color32,
    pub mp3_badge: Color32,
    pub m4a_badge: Color32,
    pub wav_badge: Color32,
    pub aiff_badge: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        Self {
            // Fastpotify dark palette
            window: Color32::from_rgb(0x0f, 0x11, 0x14),
            panel: Color32::from_rgb(0x15, 0x18, 0x1c),
            surface: Color32::from_rgb(0x1d, 0x21, 0x27),
            surface_hover: Color32::from_rgb(0x26, 0x2b, 0x33),
            surface_active: Color32::from_rgb(0x2f, 0x35, 0x3f),
            outline: Color32::from_rgb(0x2a, 0x30, 0x38),
            text: Color32::from_rgb(0xf2, 0xf4, 0xf6),
            secondary: Color32::from_rgb(0xa9, 0xb1, 0xbc),
            dim: Color32::from_rgb(0x6e, 0x77, 0x84),
            // Spotify vibrant green
            accent: Color32::from_rgb(0x1e, 0xd7, 0x60),
            accent_hover: Color32::from_rgb(0x3c, 0xe8, 0x7a),
            on_accent: Color32::from_rgb(0x0a, 0x14, 0x0e),
            danger: Color32::from_rgb(0xf5, 0x71, 0x7f),
            warning: Color32::from_rgb(0xf2, 0xb8, 0x5c),
            // Format badges
            flac_badge: Color32::from_rgb(0x8a, 0x5c, 0xf6),
            mp3_badge: Color32::from_rgb(0x3b, 0x82, 0xf6),
            m4a_badge: Color32::from_rgb(0xf5, 0x9e, 0x0b),
            wav_badge: Color32::from_rgb(0x10, 0xb9, 0x81),
            aiff_badge: Color32::from_rgb(0x06, 0xb6, 0xd4),
        }
    }
}

pub const RADIUS: f32 = 8.0;
pub const RADIUS_SMALL: f32 = 4.0;
pub const SIDEBAR_WIDTH: f32 = 220.0;
pub const TOPBAR_HEIGHT: f32 = 56.0;
pub const PLAYERBAR_HEIGHT: f32 = 68.0;

/// Configure the egui context styles for fastpotify aesthetics.
pub fn apply_theme(ctx: &egui::Context, palette: &Palette) {
    let mut visuals = egui::Visuals::dark();

    visuals.panel_fill = palette.panel;
    visuals.window_fill = palette.window;
    visuals.window_stroke = Stroke::new(1.0, palette.outline);
    visuals.window_corner_radius = CornerRadius::same(RADIUS as u8);

    // Non-interactive widgets (labels, panels)
    visuals.widgets.noninteractive.bg_fill = palette.surface;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(RADIUS_SMALL as u8);

    // Inactive (buttons, inputs)
    visuals.widgets.inactive.bg_fill = palette.surface;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(RADIUS_SMALL as u8);

    // Hovered
    visuals.widgets.hovered.bg_fill = palette.surface_hover;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(RADIUS_SMALL as u8);

    // Active
    visuals.widgets.active.bg_fill = palette.surface_active;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.accent);
    visuals.widgets.active.corner_radius = CornerRadius::same(RADIUS_SMALL as u8);

    // Text selection & cursor
    visuals.selection.bg_fill = palette.accent;
    visuals.selection.stroke = Stroke::new(1.0, palette.on_accent);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.window_margin = Margin::same(12);
    ctx.set_style(style);
}

/// Paints a crisp vector checkmark without depending on font glyphs.
pub fn paint_checkmark(ui: &mut egui::Ui, color: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let stroke = Stroke::new(2.2, color);
    let p1 = rect.min + Vec2::new(size * 0.15, size * 0.52);
    let p2 = rect.min + Vec2::new(size * 0.42, size * 0.80);
    let p3 = rect.min + Vec2::new(size * 0.88, size * 0.22);
    ui.painter().line_segment([p1, p2], stroke);
    ui.painter().line_segment([p2, p3], stroke);
}

/// Configures system fonts on Windows (Segoe UI Symbol, Segoe UI Emoji) so symbols and emojis never render as tofu boxes.
pub fn install_system_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    #[cfg(windows)]
    {
        if let Ok(bytes) = std::fs::read("C:\\Windows\\Fonts\\seguisym.ttf") {
            fonts.font_data.insert(
                "seguisym".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.push("seguisym".to_owned());
            }
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.push("seguisym".to_owned());
            }
        }
        if let Ok(bytes) = std::fs::read("C:\\Windows\\Fonts\\seguiemj.ttf") {
            fonts.font_data.insert(
                "seguiemj".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.push("seguiemj".to_owned());
            }
        }
    }

    ctx.set_fonts(fonts);
}
