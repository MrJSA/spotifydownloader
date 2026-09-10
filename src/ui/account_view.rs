use egui::{Button, CornerRadius, RichText, Stroke, TextEdit};
use std::sync::Arc;
use crate::auth::AuthManager;
use crate::theme::{Palette, RADIUS, RADIUS_SMALL};

pub fn render_account_view(
    ui: &mut egui::Ui,
    auth: &Arc<AuthManager>,
    palette: &Palette,
) {
    ui.add_space(8.0);
    ui.heading(RichText::new("Spotify Account (Optional)").color(palette.text));
    ui.add_space(6.0);
    ui.label(
        RichText::new("Sign in to download directly from Spotify CDN at 320 kbps (with Spotify Premium) or access private playlists.")
            .color(palette.dim),
    );
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(16.0);

    let is_logged_in = auth.is_logged_in();
    let profile_opt = auth.profile.lock().unwrap().clone();
    let is_logging_in = *auth.is_logging_in.lock().unwrap();
    let is_premium = auth.is_premium();

    egui::Frame::NONE
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(CornerRadius::same(RADIUS as u8))
        .inner_margin(egui::Margin::same(20))
        .show(ui, |ui| {
            if is_logged_in {
                if let Some(profile) = profile_opt {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(56.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 28.0, palette.accent);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "👤",
                            egui::FontId::proportional(28.0),
                            palette.on_accent,
                        );

                        ui.add_space(16.0);

                        ui.vertical(|ui| {
                            let name = profile.display_name.unwrap_or(profile.id);
                            ui.heading(RichText::new(name).color(palette.text).strong());
                            if let Some(email) = profile.email {
                                ui.label(RichText::new(email).color(palette.dim));
                            }
                            let has_cdn = auth.has_cdn_session();
                            if has_cdn {
                                ui.label(
                                    RichText::new("🟢 SPOTIFY PREMIUM • Direct 320 kbps Stream Active")
                                        .size(12.0)
                                        .color(palette.accent)
                                        .strong(),
                                );
                            } else if is_premium {
                                ui.label(
                                    RichText::new("🔵 SPOTIFY WEB CONNECTED • High-Bitrate Resolver Active")
                                        .size(12.0)
                                        .color(palette.secondary)
                                        .strong(),
                                );
                                ui.add_space(2.0);
                                ui.label(
                                    RichText::new("ℹ To enable direct 320 kbps CDN streaming from Spotify's servers, sign in below with Option B (Username & Password).")
                                        .size(11.0)
                                        .color(palette.dim),
                                );
                            } else {
                                ui.label(
                                    RichText::new("⚪ SPOTIFY FREE • High-Bitrate Multi-Source Resolver Active")
                                        .size(12.0)
                                        .color(palette.secondary)
                                        .strong(),
                                );
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Sign Out").clicked() {
                                let auth_clone = auth.clone();
                                tokio::spawn(async move {
                                    auth_clone.logout().await;
                                });
                            }
                        });
                    });

                    if is_premium && !auth.has_cdn_session() {
                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Direct Spotify Stream Authentication (Optional)")
                                .size(14.0)
                                .color(palette.text)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("Spotify Access Points require direct credential handshake to stream encrypted 320 kbps Vorbis.")
                                .size(11.0)
                                .color(palette.dim),
                        );
                        ui.add_space(8.0);

                        let id_user = ui.make_persistent_id("login_username_opt");
                        let id_pass = ui.make_persistent_id("login_password_opt");
                        let mut username = ui.data_mut(|d| d.get_temp::<String>(id_user).unwrap_or_default());
                        let mut password = ui.data_mut(|d| d.get_temp::<String>(id_pass).unwrap_or_default());

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Username / Email:").color(palette.dim));
                            ui.add(TextEdit::singleline(&mut username).desired_width(180.0));
                            ui.add_space(8.0);
                            ui.label(RichText::new("Password:").color(palette.dim));
                            ui.add(TextEdit::singleline(&mut password).password(true).desired_width(180.0));

                            let creds_btn = Button::new(RichText::new("Connect Direct Stream").color(palette.text).strong())
                                .fill(palette.surface_active)
                                .stroke(Stroke::new(1.0, palette.outline))
                                .corner_radius(CornerRadius::same(RADIUS_SMALL as u8));

                            if ui.add_enabled(!username.is_empty() && !password.is_empty(), creds_btn).clicked() {
                                let auth_clone = auth.clone();
                                let u = username.clone();
                                let p = password.clone();
                                tokio::spawn(async move {
                                    let _ = auth_clone.login_with_credentials(&u, &p).await;
                                });
                            }
                        });

                        ui.data_mut(|d| {
                            d.insert_temp(id_user, username);
                            d.insert_temp(id_pass, password);
                        });
                    }
                }
            } else {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Sign In to Spotify").size(18.0).color(palette.text).strong());
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Logging in is optional. You can download public tracks, albums, and playlists without logging in.")
                            .color(palette.dim),
                    );
                    ui.label(
                        RichText::new("With Spotify Premium, the app downloads directly from Spotify's servers at full 320 kbps.")
                            .color(palette.accent),
                    );

                    // Error alert if last login attempt failed
                    if let Some(err) = auth.get_last_error() {
                        ui.add_space(8.0);
                        egui::Frame::NONE
                            .fill(egui::Color32::from_rgb(45, 20, 20))
                            .stroke(Stroke::new(1.0, egui::Color32::from_rgb(180, 50, 50)))
                            .corner_radius(CornerRadius::same(RADIUS_SMALL as u8))
                            .inner_margin(egui::Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("⚠️").color(palette.danger));
                                    ui.label(RichText::new(format!("Sign in error: {}", err)).color(palette.danger).size(12.0));
                                });
                            });
                        ui.add_space(8.0);
                    }

                    ui.add_space(16.0);

                    // Method 1: Browser PKCE OAuth
                    ui.label(RichText::new("Option A: Sign in via Browser (Recommended)").size(13.0).color(palette.secondary).strong());
                    ui.add_space(6.0);
                    ui.label(RichText::new("Opens Spotify's official consent page in your default browser.").size(11.0).color(palette.dim));
                    ui.add_space(4.0);

                    let sign_in_text = if is_logging_in {
                        "Waiting for browser approval... (check your browser)"
                    } else {
                        "🟢 Sign In with Browser"
                    };

                    let sign_in_btn = Button::new(RichText::new(sign_in_text).color(palette.on_accent).strong())
                        .fill(if is_logging_in { palette.surface_active } else { palette.accent })
                        .corner_radius(CornerRadius::same(RADIUS_SMALL as u8));

                    ui.horizontal(|ui| {
                        if ui.add_enabled(!is_logging_in, sign_in_btn).clicked() {
                            let auth_clone = auth.clone();
                            tokio::spawn(async move {
                                let _ = auth_clone.start_login_flow().await;
                            });
                        }

                        if is_logging_in {
                            ui.add(egui::Spinner::new().size(18.0));
                        }
                    });

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(16.0);

                    // Method 2: Direct Spotify Username & Password
                    ui.label(RichText::new("Option B: Direct Spotify Login").size(13.0).color(palette.secondary).strong());
                    ui.add_space(4.0);
                    ui.label(RichText::new("Credentials are kept strictly local in memory to establish a secure Spirc session.").size(11.0).color(palette.dim));
                    ui.add_space(8.0);

                    let id_user = ui.make_persistent_id("login_username");
                    let id_pass = ui.make_persistent_id("login_password");
                    let mut username = ui.data_mut(|d| d.get_temp::<String>(id_user).unwrap_or_default());
                    let mut password = ui.data_mut(|d| d.get_temp::<String>(id_pass).unwrap_or_default());

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Username / Email:").color(palette.dim));
                        ui.add(TextEdit::singleline(&mut username).desired_width(200.0));
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Password:").color(palette.dim));
                        ui.add(TextEdit::singleline(&mut password).password(true).desired_width(200.0));
                    });

                    ui.add_space(8.0);

                    let creds_btn = Button::new(RichText::new("Sign In with Credentials").color(palette.text).strong())
                        .fill(palette.surface_active)
                        .stroke(Stroke::new(1.0, palette.outline))
                        .corner_radius(CornerRadius::same(RADIUS_SMALL as u8));

                    if ui.add_enabled(!username.is_empty() && !password.is_empty(), creds_btn).clicked() {
                        let auth_clone = auth.clone();
                        let u = username.clone();
                        let p = password.clone();
                        tokio::spawn(async move {
                            if let Err(e) = auth_clone.login_with_credentials(&u, &p).await {
                                eprintln!("Credentials login failed: {}", e);
                            }
                        });
                    }

                    ui.data_mut(|d| {
                        d.insert_temp(id_user, username);
                        d.insert_temp(id_pass, password);
                    });
                });
            }
        });
}
