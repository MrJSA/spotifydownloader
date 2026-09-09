use egui::{Button, CornerRadius, RichText, Stroke, Vec2};
use crate::model::{AudioFormat, CatalogAlbum, CatalogArtist, CatalogSearchResults, FetchedCollection, TrackMetadata};
use crate::theme::{Palette, RADIUS, RADIUS_SMALL};

pub enum SearchAction {
    OpenAlbum(String),
    DownloadAlbum(String),
    OpenArtist { id: String, name: String },
    BackToSearch,
    BackToArtist,
    QuickSearch(String),
    EnqueueTrack(TrackMetadata),
    DownloadAllTracks(Vec<TrackMetadata>),
}

pub fn render_search_view(
    ui: &mut egui::Ui,
    collection: &Option<FetchedCollection>,
    catalog_results: &Option<CatalogSearchResults>,
    artist_albums: &Option<(String, Vec<CatalogAlbum>)>,
    showing_collection: bool,
    showing_artist: bool,
    is_fetching: bool,
    search_error: &Option<String>,
    current_format: AudioFormat,
    palette: &Palette,
) -> Option<SearchAction> {
    let mut action = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(8.0);

            // Error banner if any search or link fetch failed
            if let Some(err) = search_error {
                ui.add_space(4.0);
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(45, 20, 20))
                    .stroke(Stroke::new(1.0, egui::Color32::from_rgb(180, 50, 50)))
                    .corner_radius(CornerRadius::same(RADIUS_SMALL as u8))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⚠️").color(palette.danger));
                            ui.label(RichText::new(err).color(palette.danger).size(13.0));
                        });
                    });
                ui.add_space(8.0);
            }

            // Loading state while network fetch is running
            if is_fetching {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.add(egui::Spinner::new().size(36.0));
                    ui.add_space(16.0);
                    ui.label(RichText::new("Loading from Spotify...").color(palette.dim).size(14.0));
                });
                return;
            }

            // 1. Detailed Collection / Album View
            if showing_collection {
                if let Some(coll) = collection {
                    action = render_collection_view(ui, coll, showing_artist, current_format, palette);
                    return;
                }
            }

            // 2. Artist Discography View
            if showing_artist {
                if let Some((artist_name, albums)) = artist_albums {
                    action = render_artist_discography_view(ui, artist_name, albums, current_format, palette);
                    return;
                }
            }

            // 3. Search Results (Albums, Artists, Songs)
            if let Some(results) = catalog_results {
                action = render_catalog_results_view(ui, results, current_format, palette);
                return;
            }

            // 4. Initial / Empty Search State
            action = render_empty_state(ui, palette);
        });

    action
}

fn render_collection_view(
    ui: &mut egui::Ui,
    coll: &FetchedCollection,
    has_artist_back: bool,
    current_format: AudioFormat,
    palette: &Palette,
) -> Option<SearchAction> {
    let mut action = None;

    // Back Navigation Button
    ui.horizontal(|ui| {
        let back_label = if has_artist_back { "← Back to Artist" } else { "← Back to Search Results" };
        let back_btn = Button::new(RichText::new(back_label).size(12.0).color(palette.text))
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(CornerRadius::same(RADIUS_SMALL as u8));

        if ui.add(back_btn).clicked() {
            action = if has_artist_back {
                Some(SearchAction::BackToArtist)
            } else {
                Some(SearchAction::BackToSearch)
            };
        }
    });

    ui.add_space(12.0);

    // Header Card
    egui::Frame::NONE
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(CornerRadius::same(RADIUS as u8))
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Cover art image or styled fallback
                if let Some(cover_url) = &coll.cover_url {
                    ui.add(
                        egui::Image::new(cover_url)
                            .fit_to_exact_size(Vec2::splat(120.0))
                            .corner_radius(CornerRadius::same(RADIUS_SMALL as u8)),
                    );
                } else {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(120.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, CornerRadius::same(RADIUS_SMALL as u8), palette.surface_hover);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "🎵",
                        egui::FontId::proportional(40.0),
                        palette.dim,
                    );
                }

                ui.add_space(16.0);

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{:?}", coll.collection_type).to_uppercase())
                            .size(11.0)
                            .color(palette.accent)
                            .strong(),
                    );
                    ui.heading(RichText::new(&coll.title).size(22.0).color(palette.text).strong());
                    ui.label(RichText::new(format!("{} • {} tracks", coll.subtitle, coll.tracks.len())).color(palette.dim));

                    ui.add_space(8.0);

                    let dl_all_btn = Button::new(
                        RichText::new(format!("⬇ Download Entire Album ({})", current_format.short_name()))
                            .size(13.0)
                            .color(palette.on_accent)
                            .strong(),
                    )
                    .fill(palette.accent)
                    .corner_radius(CornerRadius::same(RADIUS_SMALL as u8));

                    if ui.add(dl_all_btn).clicked() {
                        action = Some(SearchAction::DownloadAllTracks(coll.tracks.clone()));
                    }
                });
            });
        });

    ui.add_space(16.0);
    ui.label(RichText::new("Tracklist").size(16.0).color(palette.text).strong());
    ui.add_space(8.0);

    // Track rows
    for track in &coll.tracks {
        egui::Frame::NONE
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(CornerRadius::same(RADIUS_SMALL as u8))
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{:02}", track.track_number))
                            .color(palette.dim)
                            .monospace(),
                    );
                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.label(RichText::new(&track.title).color(palette.text).strong());
                        ui.label(RichText::new(track.artists_string()).size(11.0).color(palette.dim));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let dl_track_btn = Button::new(
                            RichText::new(format!("⬇ {}", current_format.short_name()))
                                .size(11.0)
                                .color(palette.on_accent)
                                .strong(),
                        )
                        .fill(palette.accent)
                        .corner_radius(CornerRadius::same(RADIUS_SMALL as u8));

                        if ui.add(dl_track_btn).clicked() {
                            action = Some(SearchAction::EnqueueTrack(track.clone()));
                        }

                        if track.duration_ms > 0 {
                            ui.label(RichText::new(track.duration_formatted()).size(12.0).color(palette.dim));
                        }
                    });
                });
            });
        ui.add_space(4.0);
    }

    action
}

fn render_artist_discography_view(
    ui: &mut egui::Ui,
    artist_name: &str,
    albums: &[CatalogAlbum],
    current_format: AudioFormat,
    palette: &Palette,
) -> Option<SearchAction> {
    let mut action = None;

    // Back to Search Results
    ui.horizontal(|ui| {
        let back_btn = Button::new(RichText::new("← Back to Search Results").size(12.0).color(palette.text))
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(CornerRadius::same(RADIUS_SMALL as u8));

        if ui.add(back_btn).clicked() {
            action = Some(SearchAction::BackToSearch);
        }
    });

    ui.add_space(12.0);

    ui.heading(RichText::new(format!("{}'s Discography", artist_name)).size(22.0).color(palette.text).strong());
    ui.label(RichText::new(format!("{} albums and singles available to browse or download", albums.len())).color(palette.dim));
    ui.add_space(14.0);

    // Responsive Album Grid
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);
        for album in albums {
            let (open, download) = render_album_card(ui, album, current_format, palette);
            if open {
                action = Some(SearchAction::OpenAlbum(album.id.clone()));
            } else if download {
                action = Some(SearchAction::DownloadAlbum(album.id.clone()));
            }
        }
    });

    action
}

fn render_catalog_results_view(
    ui: &mut egui::Ui,
    results: &CatalogSearchResults,
    current_format: AudioFormat,
    palette: &Palette,
) -> Option<SearchAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.heading(RichText::new(format!("Results for “{}”", results.query)).size(20.0).color(palette.text).strong());
    });
    ui.add_space(12.0);

    // 1. ALBUMS SECTION
    if !results.albums.is_empty() {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Albums").size(16.0).color(palette.text).strong());
            ui.label(RichText::new(format!("({} found)", results.albums.len())).size(12.0).color(palette.dim));
        });
        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);
            for album in &results.albums {
                let (open, download) = render_album_card(ui, album, current_format, palette);
                if open {
                    action = Some(SearchAction::OpenAlbum(album.id.clone()));
                } else if download {
                    action = Some(SearchAction::DownloadAlbum(album.id.clone()));
                }
            }
        });
        ui.add_space(20.0);
    }

    // 2. ARTISTS SECTION
    if !results.artists.is_empty() {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Artists").size(16.0).color(palette.text).strong());
            ui.label(RichText::new(format!("({} found)", results.artists.len())).size(12.0).color(palette.dim));
        });
        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);
            for artist in &results.artists {
                if render_artist_card(ui, artist, palette) {
                    action = Some(SearchAction::OpenArtist {
                        id: artist.id.clone(),
                        name: artist.name.clone(),
                    });
                }
            }
        });
        ui.add_space(20.0);
    }

    // 3. SONGS SECTION
    if !results.tracks.is_empty() {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Songs").size(16.0).color(palette.text).strong());
            ui.label(RichText::new(format!("({} found)", results.tracks.len())).size(12.0).color(palette.dim));
        });
        ui.add_space(8.0);

        for track in &results.tracks {
            egui::Frame::NONE
                .fill(palette.surface)
                .stroke(Stroke::new(1.0, palette.outline))
                .corner_radius(CornerRadius::same(RADIUS_SMALL as u8))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(cover) = &track.cover_url {
                            ui.add(
                                egui::Image::new(cover)
                                    .fit_to_exact_size(Vec2::splat(36.0))
                                    .corner_radius(CornerRadius::same(4)),
                            );
                            ui.add_space(6.0);
                        }

                        ui.vertical(|ui| {
                            ui.label(RichText::new(&track.title).color(palette.text).strong());
                            ui.label(
                                RichText::new(format!("{} • {}", track.artists_string(), track.album))
                                    .size(11.0)
                                    .color(palette.dim),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let dl_track_btn = Button::new(
                                RichText::new(format!("⬇ {}", current_format.short_name()))
                                    .size(11.0)
                                    .color(palette.on_accent)
                                    .strong(),
                            )
                            .fill(palette.accent)
                            .corner_radius(CornerRadius::same(RADIUS_SMALL as u8));

                            if ui.add(dl_track_btn).clicked() {
                                action = Some(SearchAction::EnqueueTrack(track.clone()));
                            }

                            if track.duration_ms > 0 {
                                ui.label(RichText::new(track.duration_formatted()).size(12.0).color(palette.dim));
                            }
                        });
                    });
                });
            ui.add_space(4.0);
        }
    }

    action
}

fn render_album_card(
    ui: &mut egui::Ui,
    album: &CatalogAlbum,
    current_format: AudioFormat,
    palette: &Palette,
) -> (bool, bool) {
    let mut open_clicked = false;
    let mut download_clicked = false;
    let card_width = 175.0;

    egui::Frame::NONE
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(CornerRadius::same(RADIUS_SMALL as u8))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(card_width);
            ui.vertical(|ui| {
                // Cover Art
                let cover_size = Vec2::splat(card_width);
                if let Some(url) = &album.cover_url {
                    let img_res = ui.add(
                        egui::Image::new(url)
                            .fit_to_exact_size(cover_size)
                            .corner_radius(CornerRadius::same(6)),
                    );
                    if img_res.interact(egui::Sense::click()).clicked() {
                        open_clicked = true;
                    }
                } else {
                    let (rect, res) = ui.allocate_exact_size(cover_size, egui::Sense::click());
                    ui.painter().rect_filled(rect, CornerRadius::same(6), palette.surface_hover);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "💿",
                        egui::FontId::proportional(36.0),
                        palette.dim,
                    );
                    if res.clicked() {
                        open_clicked = true;
                    }
                }

                ui.add_space(8.0);

                // Title (clickable to open)
                let title_label = egui::Label::new(
                    RichText::new(&album.title)
                        .color(palette.text)
                        .strong()
                        .size(13.0),
                ).truncate();

                if ui.add(title_label).interact(egui::Sense::click()).clicked() {
                    open_clicked = true;
                }

                // Subtitle (Artist & Year)
                let year_snippet = album.release_date.chars().take(4).collect::<String>();
                let sub = if year_snippet.is_empty() {
                    album.artist.clone()
                } else {
                    format!("{} • {}", year_snippet, album.artist)
                };
                ui.add(egui::Label::new(RichText::new(sub).size(11.0).color(palette.dim)).truncate());

                if album.total_tracks > 0 {
                    ui.label(RichText::new(format!("{} tracks", album.total_tracks)).size(10.5).color(palette.secondary));
                }

                ui.add_space(6.0);

                // Direct 1-Click Download Button
                let dl_btn = Button::new(
                    RichText::new(format!("⬇ Download ({})", current_format.short_name()))
                        .size(11.0)
                        .color(palette.on_accent)
                        .strong(),
                )
                .fill(palette.accent)
                .corner_radius(CornerRadius::same(RADIUS_SMALL as u8));

                if ui.add_sized([card_width, 24.0], dl_btn).clicked() {
                    download_clicked = true;
                }
            });
        });

    (open_clicked, download_clicked)
}

fn render_artist_card(
    ui: &mut egui::Ui,
    artist: &CatalogArtist,
    palette: &Palette,
) -> bool {
    let mut clicked = false;
    let card_width = 150.0;

    let res = egui::Frame::NONE
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(CornerRadius::same(RADIUS_SMALL as u8))
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(card_width);
            ui.vertical_centered(|ui| {
                let avatar_size = Vec2::splat(100.0);
                if let Some(url) = &artist.image_url {
                    ui.add(
                        egui::Image::new(url)
                            .fit_to_exact_size(avatar_size)
                            .corner_radius(CornerRadius::same(50)),
                    );
                } else {
                    let (rect, _) = ui.allocate_exact_size(avatar_size, egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 50.0, palette.surface_hover);
                    let initial = artist.name.chars().next().unwrap_or('A').to_uppercase().to_string();
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        initial,
                        egui::FontId::proportional(32.0),
                        palette.dim,
                    );
                }

                ui.add_space(8.0);
                ui.add(egui::Label::new(RichText::new(&artist.name).color(palette.text).strong().size(13.0)).truncate());
                ui.label(RichText::new("Artist • View albums").size(11.0).color(palette.accent));
            });
        });

    if res.response.interact(egui::Sense::click()).clicked() {
        clicked = true;
    }

    clicked
}

fn render_empty_state(ui: &mut egui::Ui, palette: &Palette) -> Option<SearchAction> {
    let mut action = None;

    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.label(RichText::new("🔍").size(48.0));
        ui.add_space(10.0);
        ui.label(
            RichText::new("Search Spotify or Paste a Link")
                .size(20.0)
                .color(palette.text)
                .strong(),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("Browse millions of songs, albums, and artists. Click any album to view tracks or download directly.")
                .color(palette.dim),
        );

        ui.add_space(32.0);
        ui.label(RichText::new("Popular Artists to Browse").size(14.0).color(palette.secondary).strong());
        ui.add_space(10.0);

        let popular = [
            "The Weeknd",
            "Taylor Swift",
            "Daft Punk",
            "Billie Eilish",
            "Coldplay",
            "Kendrick Lamar",
            "Eminem",
            "Linkin Park",
            "Ed Sheeran",
            "Queen",
        ];

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
            for artist in popular {
                let btn = Button::new(RichText::new(artist).size(12.0).color(palette.text))
                    .fill(palette.surface)
                    .stroke(Stroke::new(1.0, palette.outline))
                    .corner_radius(CornerRadius::same(14));

                if ui.add(btn).clicked() {
                    action = Some(SearchAction::QuickSearch(artist.to_string()));
                }
            }
        });
    });

    action
}
