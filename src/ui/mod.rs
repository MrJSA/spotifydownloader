pub mod account_view;
pub mod library_view;
pub mod player_bar;
pub mod queue_view;
pub mod search_view;
pub mod settings_view;
pub mod sidebar;
pub mod topbar;

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::auth::AuthManager;
use crate::downloader::{DownloadEvent, DownloadManager};
use crate::model::{AudioFormat, CatalogAlbum, CatalogSearchResults, DownloadStatus, FetchedCollection, UserSettings};
use crate::spotify::{SpotifyClient, SpotifyLink};
use crate::theme::{Palette, apply_theme};
use self::sidebar::NavigationTab;

pub enum AsyncUiResult {
    CollectionFetched(anyhow::Result<FetchedCollection>),
    CatalogSearched(anyhow::Result<CatalogSearchResults>),
    ArtistAlbumsFetched {
        name: String,
        result: anyhow::Result<Vec<CatalogAlbum>>,
    },
    AlbumDownloaded(anyhow::Result<FetchedCollection>),
}

pub struct SpotifyDownloaderApp {
    pub palette: Palette,
    pub current_tab: NavigationTab,
    pub url_input: String,
    pub current_format: AudioFormat,
    pub is_fetching: bool,

    // Search and Catalog Browse state
    pub fetched_collection: Option<FetchedCollection>,
    pub catalog_results: Option<CatalogSearchResults>,
    pub artist_albums: Option<(String, Vec<CatalogAlbum>)>,
    pub showing_collection: bool,
    pub showing_artist: bool,
    pub search_error: Option<String>,

    pub settings: Arc<std::sync::Mutex<UserSettings>>,
    pub auth: Arc<AuthManager>,
    pub spotify: Arc<SpotifyClient>,
    pub manager: Arc<DownloadManager>,
    pub rx: mpsc::UnboundedReceiver<DownloadEvent>,
    pub async_tx: mpsc::UnboundedSender<AsyncUiResult>,
    pub async_rx: mpsc::UnboundedReceiver<AsyncUiResult>,
}

impl SpotifyDownloaderApp {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (async_tx, async_rx) = mpsc::unbounded_channel();
        let loaded_settings = UserSettings::load();
        let default_format = loaded_settings.default_format;
        let settings = Arc::new(std::sync::Mutex::new(loaded_settings));
        let auth = Arc::new(AuthManager::new());
        // Load cached credentials synchronously so the UI displays the profile immediately
        auth.load_persisted_cached();
        // Background async refresh and session connect
        let auth_clone = auth.clone();
        tokio::spawn(async move {
            auth_clone.load_persisted().await;
        });

        let spotify = Arc::new(SpotifyClient::new(auth.clone()));
        let manager = Arc::new(DownloadManager::new(
            settings.clone(),
            auth.clone(),
            spotify.clone(),
            tx,
        ));

        let palette = Palette::dark();

        Self {
            palette,
            current_tab: NavigationTab::Search,
            url_input: String::new(),
            current_format: default_format,
            is_fetching: false,
            fetched_collection: None,
            catalog_results: None,
            artist_albums: None,
            showing_collection: false,
            showing_artist: false,
            search_error: None,
            settings,
            auth,
            spotify,
            manager,
            rx,
            async_tx,
            async_rx,
        }
    }

    fn poll_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                DownloadEvent::StatusChanged { id, status, destination_path } => {
                    if let Ok(mut queue) = self.manager.queue.lock() {
                        if let Some(item) = queue.iter_mut().find(|i| i.id == id) {
                            item.status = status;
                            if destination_path.is_some() {
                                item.destination_path = destination_path;
                            }
                        }
                    }
                }
                DownloadEvent::ItemFinished { .. } => {}
            }
        }

        while let Ok(res) = self.async_rx.try_recv() {
            self.is_fetching = false;
            match res {
                AsyncUiResult::CollectionFetched(Ok(collection)) => {
                    self.fetched_collection = Some(collection);
                    self.showing_collection = true;
                    self.search_error = None;
                }
                AsyncUiResult::CollectionFetched(Err(err)) => {
                    eprintln!("Collection fetch error: {}", err);
                    self.search_error = Some(format!("Could not load Spotify link: {}", err));
                    self.showing_collection = false;
                }
                AsyncUiResult::CatalogSearched(Ok(results)) => {
                    self.catalog_results = Some(results);
                    self.showing_collection = false;
                    self.showing_artist = false;
                    self.search_error = None;
                }
                AsyncUiResult::CatalogSearched(Err(err)) => {
                    eprintln!("Catalog search error: {}", err);
                    self.search_error = Some(format!("Search error: {}", err));
                }
                AsyncUiResult::ArtistAlbumsFetched { name, result: Ok(albums) } => {
                    self.artist_albums = Some((name, albums));
                    self.showing_artist = true;
                    self.showing_collection = false;
                    self.search_error = None;
                }
                AsyncUiResult::ArtistAlbumsFetched { result: Err(err), .. } => {
                    eprintln!("Artist albums error: {}", err);
                    self.search_error = Some(format!("Failed to load artist: {}", err));
                }
                AsyncUiResult::AlbumDownloaded(Ok(collection)) => {
                    for track in collection.tracks {
                        self.manager.enqueue(track, self.current_format);
                    }
                }
                AsyncUiResult::AlbumDownloaded(Err(err)) => {
                    eprintln!("Direct album download error: {}", err);
                    self.search_error = Some(format!("Album download error: {}", err));
                }
            }
        }
    }

    pub fn trigger_search(&mut self) {
        let query = self.url_input.trim().to_string();
        if query.is_empty() {
            return;
        }

        self.is_fetching = true;
        self.search_error = None;
        self.current_tab = NavigationTab::Search;

        let is_link = query.contains("spotify.com")
            || query.contains("spotify.link")
            || query.contains("spotify:")
            || query.contains("spoti.fi")
            || query.starts_with("http://")
            || query.starts_with("https://");

        if is_link {
            self.showing_collection = true;
            self.showing_artist = false;
            let spotify = self.spotify.clone();
            let tx = self.async_tx.clone();

            tokio::spawn(async move {
                if let Some(link) = spotify.resolve_spotify_link(&query).await {
                    match link {
                        SpotifyLink::Artist(artist_id) => {
                            let res = spotify.fetch_artist_albums(&artist_id).await;
                            let _ = tx.send(AsyncUiResult::ArtistAlbumsFetched {
                                name: "Artist Discography".to_string(),
                                result: res,
                            });
                        }
                        other => {
                            let res = spotify.fetch_link(&other).await;
                            let _ = tx.send(AsyncUiResult::CollectionFetched(res));
                        }
                    }
                } else {
                    let _ = tx.send(AsyncUiResult::CollectionFetched(Err(anyhow::anyhow!(
                        "Unrecognized or invalid Spotify link. Please check the URL and try again."
                    ))));
                }
            });
        } else {
            // Text search query
            self.showing_collection = false;
            self.showing_artist = false;
            let spotify = self.spotify.clone();
            let tx = self.async_tx.clone();

            tokio::spawn(async move {
                let res = spotify.search_catalog(&query).await;
                let _ = tx.send(AsyncUiResult::CatalogSearched(res));
            });
        }
    }
}

impl eframe::App for SpotifyDownloaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events();
        apply_theme(ctx, &self.palette);

        // Continuous repaint if items are downloading or search is in flight
        let has_active_downloads = self.manager.queue.lock().map(|q| {
            q.iter().any(|i| i.status.is_active())
        }).unwrap_or(false);

        if has_active_downloads || self.is_fetching {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // Top Bar
        let user_name = self.auth.get_user_name();
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::NONE.fill(self.palette.panel).inner_margin(egui::Margin::symmetric(16, 8)))
            .show(ctx, |ui| {
                let prev_format = self.current_format;
                let (fetch_clicked, account_clicked) = topbar::render_topbar(
                    ui,
                    &mut self.url_input,
                    &mut self.current_format,
                    self.is_fetching,
                    user_name.as_deref(),
                    &self.palette,
                );

                if self.current_format != prev_format {
                    if let Ok(mut s) = self.settings.lock() {
                        s.default_format = self.current_format;
                        let _ = s.save();
                    }
                }

                if account_clicked {
                    self.current_tab = NavigationTab::Account;
                }

                if fetch_clicked && !self.url_input.trim().is_empty() {
                    self.trigger_search();
                }
            });

        // Bottom Player / Progress Bar
        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(egui::Frame::NONE.fill(self.palette.panel).inner_margin(egui::Margin::symmetric(16, 8)))
            .show(ctx, |ui| {
                player_bar::render_player_bar(ui, &self.manager, &self.palette);
            });

        // Left Navigation Sidebar
        let queue_count = self.manager.queue.lock().map(|q| {
            q.iter().filter(|i| i.status.is_active() || matches!(i.status, DownloadStatus::Queued)).count()
        }).unwrap_or(0);

        egui::SidePanel::left("left_sidebar")
            .frame(egui::Frame::NONE.fill(self.palette.panel).inner_margin(egui::Margin::same(12)))
            .resizable(false)
            .show(ctx, |ui| {
                sidebar::render_sidebar(ui, &mut self.current_tab, queue_count, &self.palette);
            });

        // Central Dynamic Panel
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(self.palette.window).inner_margin(egui::Margin::same(16)))
            .show(ctx, |ui| {
                match self.current_tab {
                    NavigationTab::Queue => {
                        queue_view::render_queue_view(ui, &self.manager, &self.palette);
                    }
                    NavigationTab::Search => {
                        if let Some(action) = search_view::render_search_view(
                            ui,
                            &self.fetched_collection,
                            &self.catalog_results,
                            &self.artist_albums,
                            self.showing_collection,
                            self.showing_artist,
                            self.is_fetching,
                            &self.search_error,
                            self.current_format,
                            &self.palette,
                        ) {
                            match action {
                                search_view::SearchAction::OpenAlbum(id) => {
                                    self.is_fetching = true;
                                    let spotify = self.spotify.clone();
                                    let tx = self.async_tx.clone();
                                    tokio::spawn(async move {
                                        let res = spotify.fetch_album_by_id(&id).await;
                                        let _ = tx.send(AsyncUiResult::CollectionFetched(res));
                                    });
                                }
                                search_view::SearchAction::DownloadAlbum(id) => {
                                    self.is_fetching = true;
                                    let spotify = self.spotify.clone();
                                    let tx = self.async_tx.clone();
                                    tokio::spawn(async move {
                                        let res = spotify.fetch_album_by_id(&id).await;
                                        let _ = tx.send(AsyncUiResult::AlbumDownloaded(res));
                                    });
                                }
                                search_view::SearchAction::OpenArtist { id, name } => {
                                    self.is_fetching = true;
                                    let spotify = self.spotify.clone();
                                    let tx = self.async_tx.clone();
                                    tokio::spawn(async move {
                                        let res = spotify.fetch_artist_albums(&id).await;
                                        let _ = tx.send(AsyncUiResult::ArtistAlbumsFetched { name, result: res });
                                    });
                                }
                                search_view::SearchAction::BackToSearch => {
                                    self.showing_collection = false;
                                    self.showing_artist = false;
                                }
                                search_view::SearchAction::BackToArtist => {
                                    self.showing_collection = false;
                                    self.showing_artist = true;
                                }
                                search_view::SearchAction::QuickSearch(query) => {
                                    self.url_input = query;
                                    self.trigger_search();
                                }
                                search_view::SearchAction::EnqueueTrack(track) => {
                                    self.manager.enqueue(track, self.current_format);
                                }
                                search_view::SearchAction::DownloadAllTracks(tracks) => {
                                    for track in tracks {
                                        self.manager.enqueue(track, self.current_format);
                                    }
                                }
                            }
                        }
                    }
                    NavigationTab::Account => {
                        account_view::render_account_view(ui, &self.auth, &self.palette);
                    }
                    NavigationTab::Library => {
                        let settings = self.settings.lock().unwrap().clone();
                        library_view::render_library_view(ui, &settings, &self.palette);
                    }
                    NavigationTab::Settings => {
                        let old_settings = self.settings.lock().unwrap().clone();
                        let mut settings = old_settings.clone();
                        settings_view::render_settings_view(ui, &mut settings, &self.palette);
                        if settings != old_settings {
                            self.current_format = settings.default_format;
                            let _ = settings.save();
                            *self.settings.lock().unwrap() = settings;
                        }
                    }
                }
            });
    }
}
