use anyhow::{Result, bail};
use regex::Regex;
use serde_json::Value;
use std::sync::Arc;
use crate::auth::AuthManager;
use crate::model::{CollectionType, FetchedCollection, TrackMetadata};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpotifyLink {
    Track(String),
    Album(String),
    Playlist(String),
    Artist(String),
}

/// Parses a Spotify URL or URI into its link type and ID.
pub fn parse_spotify_link(input: &str) -> Option<SpotifyLink> {
    let input = input.trim().trim_matches('"').trim_matches('\'').trim_matches('<').trim_matches('>');

    // Standard, Localized, Embed, and User URLs:
    // https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT
    // https://open.spotify.com/intl-de/track/4cOdK2wGLETKBW3PvgPWqT?si=abc
    // https://open.spotify.com/intl-en/album/1DFixLWuPkv3KT3TnV35m3
    // https://open.spotify.com/de/album/1DFixLWuPkv3KT3TnV35m3
    // https://open.spotify.com/embed/album/1DFixLWuPkv3KT3TnV35m3
    // https://open.spotify.com/user/spotify/playlist/37i9dQZF1DXcBWIGoYBM5M
    // https://open.spotify.com/artist/6s2tKG7vDTgCLpCG4nlIX5
    let url_re = Regex::new(r"(?i)(?:open\.)?spotify\.com/(?:[^/\s\?]+/)*(track|album|playlist|artist)/([a-zA-Z0-9]{15,35})").ok()?;
    if let Some(caps) = url_re.captures(input) {
        let kind = caps.get(1)?.as_str().to_lowercase();
        let id = caps.get(2)?.as_str().to_string();
        return match kind.as_str() {
            "track" => Some(SpotifyLink::Track(id)),
            "album" => Some(SpotifyLink::Album(id)),
            "playlist" => Some(SpotifyLink::Playlist(id)),
            "artist" => Some(SpotifyLink::Artist(id)),
            _ => None,
        };
    }

    // Spotify URIs: spotify:track:4cOdK2wGLETKBW3PvgPWqT, spotify:artist:..., etc.
    let uri_re = Regex::new(r"(?i)spotify:(track|album|playlist|artist):([a-zA-Z0-9]{15,35})").ok()?;
    if let Some(caps) = uri_re.captures(input) {
        let kind = caps.get(1)?.as_str().to_lowercase();
        let id = caps.get(2)?.as_str().to_string();
        return match kind.as_str() {
            "track" => Some(SpotifyLink::Track(id)),
            "album" => Some(SpotifyLink::Album(id)),
            "playlist" => Some(SpotifyLink::Playlist(id)),
            "artist" => Some(SpotifyLink::Artist(id)),
            _ => None,
        };
    }

    None
}

pub struct SpotifyClient {
    http: reqwest::Client,
    auth: Arc<AuthManager>,
    anon_token: std::sync::Arc<std::sync::Mutex<Option<(String, u64)>>>,
}

impl SpotifyClient {
    pub fn new(auth: Arc<AuthManager>) -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
                .build()
                .unwrap_or_default(),
            auth,
            anon_token: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    /// Resolves any Spotify input (including shortlinks like spotify.link/...) into a SpotifyLink.
    pub async fn resolve_spotify_link(&self, input: &str) -> Option<SpotifyLink> {
        let trimmed = input.trim().trim_matches('"').trim_matches('\'').trim_matches('<').trim_matches('>');
        if let Some(link) = parse_spotify_link(trimmed) {
            return Some(link);
        }

        // Handle shortlinks: spotify.link, spoti.fi, spotify.app.link
        if trimmed.contains("spotify.link") || trimmed.contains("spoti.fi") || trimmed.contains("spotify.app.link") {
            let re = Regex::new(r"https?://(?:[a-zA-Z0-9_-]+\.)?(?:spotify\.link|spoti\.fi|spotify\.app\.link)/[a-zA-Z0-9_-]+").ok()?;
            if let Some(mat) = re.find(trimmed) {
                let short_url = mat.as_str();
                if let Ok(resp) = self.http.get(short_url).send().await {
                    let final_url = resp.url().as_str();
                    if let Some(link) = parse_spotify_link(final_url) {
                        return Some(link);
                    }
                }
            }
        }

        None
    }

    /// Obtains a Spotify Web API access token (either user's logged-in token or fresh anonymous web player token).
    pub async fn get_token(&self) -> Option<String> {
        if let Some(user_token) = self.auth.get_access_token() {
            return Some(user_token);
        }

        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        if let Ok(guard) = self.anon_token.lock() {
            if let Some((token, expiry)) = &*guard {
                if now < *expiry {
                    return Some(token.clone());
                }
            }
        }

        // 1. Try Spotify Web Player access token endpoint
        let wp_url = "https://open.spotify.com/get_access_token?reason=transport&productType=web_player";
        if let Ok(resp) = self.http.get(wp_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send().await {
            if let Ok(data) = resp.json::<Value>().await {
                if let Some(token) = data["accessToken"].as_str() {
                    let tok_str = token.to_string();
                    if let Ok(mut guard) = self.anon_token.lock() {
                        *guard = Some((tok_str.clone(), now + 3000));
                    }
                    return Some(tok_str);
                }
            }
        }

        // 2. Fallback: Fetch fresh anonymous token from Spotify embed page
        let url = "https://open.spotify.com/embed/playlist/37i9dQZF1DXcBWIGoYBM5M";
        if let Ok(resp) = self.http.get(url).send().await {
            if let Ok(html) = resp.text().await {
                let re = Regex::new(r#"<script id="__NEXT_DATA__"[^>]*>(.*?)</script>"#).unwrap();
                if let Some(caps) = re.captures(&html) {
                    if let Some(json_str) = caps.get(1) {
                        if let Ok(data) = serde_json::from_str::<Value>(json_str.as_str()) {
                            let token_opt = data.pointer("/props/pageProps/state/settings/session/accessToken")
                                .or_else(|| data.pointer("/props/pageProps/settings/session/accessToken"))
                                .or_else(|| data.pointer("/props/pageProps/session/accessToken"))
                                .and_then(|t| t.as_str());

                            if let Some(token) = token_opt {
                                let tok_str = token.to_string();
                                if let Ok(mut guard) = self.anon_token.lock() {
                                    *guard = Some((tok_str.clone(), now + 3000));
                                }
                                return Some(tok_str);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Searches Spotify catalog for albums, artists, and tracks matching the query.
    pub async fn search_catalog(&self, query: &str) -> Result<crate::model::CatalogSearchResults> {
        let mut results = crate::model::CatalogSearchResults {
            query: query.to_string(),
            ..Default::default()
        };

        // If authenticated with Spotify account, query Spotify Web API
        if let Some(token) = self.auth.get_access_token() {
            let search_url = format!(
                "https://api.spotify.com/v1/search?q={}&type=album,artist,track&limit=15",
                urlencoding::encode(query)
            );

            if let Ok(resp) = self.http.get(&search_url).bearer_auth(&token).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<Value>().await {
                    // Parse Albums
                    if let Some(albums) = data["albums"]["items"].as_array() {
                        for a in albums {
                            let id = a["id"].as_str().unwrap_or("").to_string();
                            let title = a["name"].as_str().unwrap_or("").to_string();
                            let artist = a["artists"].as_array()
                                .and_then(|arr| arr.first())
                                .and_then(|art| art["name"].as_str())
                                .unwrap_or("Unknown Artist")
                                .to_string();
                            let release_date = a["release_date"].as_str().unwrap_or("").to_string();
                            let total_tracks = a["total_tracks"].as_u64().unwrap_or(0) as u32;
                            let cover_url = a["images"].as_array()
                                .and_then(|arr| arr.first())
                                .and_then(|img| img["url"].as_str())
                                .map(str::to_string);

                            if !id.is_empty() && !title.is_empty() {
                                results.albums.push(crate::model::CatalogAlbum {
                                    id,
                                    title,
                                    artist,
                                    release_date,
                                    total_tracks,
                                    cover_url,
                                });
                            }
                        }
                    }

                    // Parse Artists
                    if let Some(artists) = data["artists"]["items"].as_array() {
                        for art in artists {
                            let id = art["id"].as_str().unwrap_or("").to_string();
                            let name = art["name"].as_str().unwrap_or("").to_string();
                            let image_url = art["images"].as_array()
                                .and_then(|arr| arr.first())
                                .and_then(|img| img["url"].as_str())
                                .map(str::to_string);

                            if !id.is_empty() && !name.is_empty() {
                                results.artists.push(crate::model::CatalogArtist {
                                    id,
                                    name,
                                    image_url,
                                });
                            }
                        }
                    }

                    // Parse Tracks
                    if let Some(tracks) = data["tracks"]["items"].as_array() {
                        for t in tracks {
                            if let Ok(track_meta) = parse_api_track(t) {
                                results.tracks.push(track_meta);
                            }
                        }
                    }

                    if !results.albums.is_empty() || !results.artists.is_empty() || !results.tracks.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }
    }

        // Fallback search via iTunes catalog (works worldwide without any tokens)
        let itunes_url = format!(
            "https://itunes.apple.com/search?term={}&entity=album,musicArtist,song&limit=20",
            urlencoding::encode(query)
        );

        if let Ok(resp) = self.http.get(&itunes_url).send().await {
            if let Ok(data) = resp.json::<Value>().await {
                if let Some(items) = data["results"].as_array() {
                    for item in items {
                        let wrapper = item["wrapperType"].as_str().unwrap_or("");
                        if wrapper == "collection" {
                            let title = item["collectionName"].as_str().unwrap_or("").to_string();
                            let artist = item["artistName"].as_str().unwrap_or("").to_string();
                            let release_date = item["releaseDate"].as_str().unwrap_or("").to_string();
                            let total_tracks = item["trackCount"].as_u64().unwrap_or(0) as u32;
                            let cover = item["artworkUrl100"].as_str().map(|u| u.replace("100x100bb", "600x600bb"));
                            let col_id = item["collectionId"].as_u64().unwrap_or(0).to_string();

                            results.albums.push(crate::model::CatalogAlbum {
                                id: format!("itunes:{}", col_id),
                                title,
                                artist,
                                release_date,
                                total_tracks,
                                cover_url: cover,
                            });
                        } else if wrapper == "artist" {
                            let name = item["artistName"].as_str().unwrap_or("").to_string();
                            let id = item["artistId"].as_u64().unwrap_or(0).to_string();
                            if !id.is_empty() && !name.is_empty() {
                                results.artists.push(crate::model::CatalogArtist {
                                    id: format!("itunes:{}", id),
                                    name,
                                    image_url: None,
                                });
                            }
                        } else if wrapper == "track" {
                            let title = item["trackName"].as_str().unwrap_or("").to_string();
                            let artist = item["artistName"].as_str().unwrap_or("").to_string();
                            let album = item["collectionName"].as_str().unwrap_or("").to_string();
                            let duration_ms = item["trackTimeMillis"].as_u64().unwrap_or(0);
                            let track_number = item["trackNumber"].as_u64().unwrap_or(1) as u32;
                            let cover_url = item["artworkUrl100"].as_str().map(|u| u.replace("100x100bb", "600x600bb"));
                            let genre = item["primaryGenreName"].as_str().map(str::to_string);
                            let id = item["trackId"].as_u64().unwrap_or(0).to_string();

                            results.tracks.push(crate::model::TrackMetadata {
                                id: id.clone(),
                                title,
                                artists: vec![artist.clone()],
                                album_artist: Some(artist),
                                album,
                                duration_ms,
                                track_number,
                                disc_number: 1,
                                release_date: "".to_string(),
                                genre,
                                isrc: None,
                                cover_url,
                                spotify_url: format!("https://open.spotify.com/track/{}", id),
                            });
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Fetches all albums by an artist.
    pub async fn fetch_artist_albums(&self, artist_id: &str) -> Result<Vec<crate::model::CatalogAlbum>> {
        let mut albums_list = Vec::new();

        if let Some(clean_id) = artist_id.strip_prefix("itunes:") {
            let url = format!("https://itunes.apple.com/lookup?id={}&entity=album&limit=30", clean_id);
            if let Ok(resp) = self.http.get(&url).send().await {
                if let Ok(data) = resp.json::<Value>().await {
                    if let Some(items) = data["results"].as_array() {
                        for a in items {
                            if a["wrapperType"].as_str() == Some("collection") {
                                let id = a["collectionId"].as_u64().unwrap_or(0).to_string();
                                let title = a["collectionName"].as_str().unwrap_or("").to_string();
                                let artist = a["artistName"].as_str().unwrap_or("Artist").to_string();
                                let release_date = a["releaseDate"].as_str().unwrap_or("").to_string();
                                let total_tracks = a["trackCount"].as_u64().unwrap_or(0) as u32;
                                let cover_url = a["artworkUrl100"].as_str().map(|u| u.replace("100x100bb", "600x600bb"));

                                albums_list.push(crate::model::CatalogAlbum {
                                    id: format!("itunes:{}", id),
                                    title,
                                    artist,
                                    release_date,
                                    total_tracks,
                                    cover_url,
                                });
                            }
                        }
                    }
                }
            }
            return Ok(albums_list);
        }

        let token = self.auth.get_access_token();
        if let Some(token) = token {
            let url = format!(
                "https://api.spotify.com/v1/artists/{}/albums?include_groups=album,single&limit=30",
                artist_id
            );

            if let Ok(resp) = self.http.get(&url).bearer_auth(&token).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<Value>().await {
                    if let Some(items) = data["items"].as_array() {
                        for a in items {
                            let id = a["id"].as_str().unwrap_or("").to_string();
                            let title = a["name"].as_str().unwrap_or("").to_string();
                            let artist = a["artists"].as_array()
                                .and_then(|arr| arr.first())
                                .and_then(|art| art["name"].as_str())
                                .unwrap_or("Artist")
                                .to_string();
                            let release_date = a["release_date"].as_str().unwrap_or("").to_string();
                            let total_tracks = a["total_tracks"].as_u64().unwrap_or(0) as u32;
                            let cover_url = a["images"].as_array()
                                .and_then(|arr| arr.first())
                                .and_then(|img| img["url"].as_str())
                                .map(str::to_string);

                            albums_list.push(crate::model::CatalogAlbum {
                                id,
                                title,
                                artist,
                                release_date,
                                total_tracks,
                                cover_url,
                            });
                        }
                    }
                }
            }
        }
    }

        Ok(albums_list)
    }

    /// Fetches an album collection by its id (Spotify or iTunes).
    pub async fn fetch_album_by_id(&self, album_id: &str) -> Result<FetchedCollection> {
        if let Some(clean_id) = album_id.strip_prefix("itunes:") {
            let lookup_url = format!("https://itunes.apple.com/lookup?id={}&entity=song", clean_id);
            if let Ok(resp) = self.http.get(&lookup_url).send().await {
                if let Ok(data) = resp.json::<Value>().await {
                    if let Some(results) = data["results"].as_array() {
                        let album_info = results.iter().find(|r| r["wrapperType"].as_str() == Some("collection"));
                        let title = album_info
                            .and_then(|a| a["collectionName"].as_str())
                            .unwrap_or("Album")
                            .to_string();
                        let artist = album_info
                            .and_then(|a| a["artistName"].as_str())
                            .unwrap_or("Artist")
                            .to_string();
                        let cover_url = album_info
                            .and_then(|a| a["artworkUrl100"].as_str())
                            .map(|u| u.replace("100x100bb", "600x600bb"));

                        let mut tracks = Vec::new();
                        let mut track_counter = 1;
                        for item in results {
                            if item["wrapperType"].as_str() == Some("track") {
                                let track_name = item["trackName"].as_str().unwrap_or("").to_string();
                                let track_num = track_counter;
                                track_counter += 1;
                                let disc_num = item["discNumber"].as_u64().unwrap_or(1) as u32;
                                let dur_ms = item["trackTimeMillis"].as_u64().unwrap_or(0);
                                let track_artist = item["artistName"].as_str().unwrap_or(&artist).to_string();
                                let release = item["releaseDate"].as_str().unwrap_or("").to_string();
                                let genre = item["primaryGenreName"].as_str().map(str::to_string);

                                tracks.push(crate::model::TrackMetadata {
                                    id: item["trackId"].as_u64().unwrap_or(0).to_string(),
                                    title: track_name,
                                    artists: vec![track_artist],
                                    album_artist: Some(artist.clone()),
                                    album: title.clone(),
                                    duration_ms: dur_ms,
                                    track_number: track_num,
                                    disc_number: disc_num,
                                    release_date: release,
                                    genre,
                                    isrc: None,
                                    cover_url: cover_url.clone(),
                                    spotify_url: format!("https://open.spotify.com/track/{}", item["trackId"].as_u64().unwrap_or(0)),
                                });
                            }
                        }

                        if !tracks.is_empty() {
                            return Ok(FetchedCollection {
                                id: album_id.to_string(),
                                title,
                                subtitle: artist,
                                cover_url,
                                tracks,
                                collection_type: crate::model::CollectionType::Album,
                            });
                        }
                    }
                }
            }
        }

        self.fetch_link(&SpotifyLink::Album(album_id.to_string())).await
    }

    /// Fetches track, album, or playlist metadata.
    pub async fn fetch_link(&self, link: &SpotifyLink) -> Result<FetchedCollection> {
        // 1. If user is signed in with OAuth, try official Spotify Web API
        if let Some(token) = self.auth.get_access_token() {
            let api_res = match link {
                SpotifyLink::Track(id) => self.fetch_track_api(id, &token).await,
                SpotifyLink::Album(id) => self.fetch_album_api(id, &token).await,
                SpotifyLink::Playlist(id) => self.fetch_playlist_api(id, &token).await,
                SpotifyLink::Artist(_) => bail!("Artists should be fetched via fetch_artist_albums"),
            };

            if let Ok(coll) = api_res {
                if !coll.tracks.is_empty() {
                    return Ok(coll);
                }
            }
        }

        // 2. Public embed resolver (ultra-reliable, fast, no login required)
        match link {
            SpotifyLink::Track(id) => self.fetch_track_public(id).await,
            SpotifyLink::Album(id) => self.fetch_album_public(id).await,
            SpotifyLink::Playlist(id) => self.fetch_playlist_public(id).await,
            SpotifyLink::Artist(_) => bail!("Artists should be fetched via fetch_artist_albums"),
        }
    }

    // Authenticated API methods
    async fn fetch_track_api(&self, id: &str, token: &str) -> Result<FetchedCollection> {
        let url = format!("https://api.spotify.com/v1/tracks/{}", id);
        let resp_res = self.http.get(&url).bearer_auth(token).send().await?;
        if !resp_res.status().is_success() {
            bail!("Spotify API returned status {}", resp_res.status());
        }
        let resp: Value = resp_res.json().await?;
        if let Some(err) = resp.get("error") {
            bail!("Spotify API error: {:?}", err);
        }

        let track = parse_api_track(&resp)?;
        if track.id.is_empty() {
            bail!("Invalid track data from Spotify API");
        }
        Ok(FetchedCollection {
            id: id.to_string(),
            title: track.title.clone(),
            subtitle: track.artists_string(),
            cover_url: track.cover_url.clone(),
            tracks: vec![track],
            collection_type: CollectionType::Track,
        })
    }

    async fn fetch_album_api(&self, id: &str, token: &str) -> Result<FetchedCollection> {
        let url = format!("https://api.spotify.com/v1/albums/{}", id);
        let resp_res = self.http.get(&url).bearer_auth(token).send().await?;
        if !resp_res.status().is_success() {
            bail!("Spotify API returned status {}", resp_res.status());
        }
        let resp: Value = resp_res.json().await?;
        if let Some(err) = resp.get("error") {
            bail!("Spotify API error: {:?}", err);
        }

        let album_name = resp["name"].as_str().unwrap_or("Unknown Album").to_string();
        let artist_name = resp["artists"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|a| a["name"].as_str())
            .unwrap_or("Unknown Artist")
            .to_string();
        let release_date = resp["release_date"].as_str().unwrap_or("").to_string();
        let cover_url = resp["images"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|i| i["url"].as_str())
            .map(str::to_string);

        let mut raw_items = resp["tracks"]["items"].as_array().cloned().unwrap_or_default();
        let mut next_url = resp["tracks"]["next"].as_str().map(str::to_string);

        while let Some(ref url) = next_url {
            if let Ok(page_resp) = self.http.get(url).bearer_auth(token).send().await {
                if let Ok(page_val) = page_resp.json::<Value>().await {
                    if let Some(items) = page_val["items"].as_array() {
                        raw_items.extend(items.clone());
                    }
                    next_url = page_val["next"].as_str().map(str::to_string);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if raw_items.is_empty() {
            bail!("No tracks returned from Spotify API for album {}", id);
        }

        let album_genre = resp["genres"]
            .as_array()
            .and_then(|g| g.first())
            .and_then(|v| v.as_str())
            .map(str::to_string);

        let mut tracks = Vec::new();
        for (idx, item) in raw_items.iter().enumerate() {
            let artists: Vec<String> = item["artists"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|a| a["name"].as_str().map(str::to_string))
                .collect();

            tracks.push(TrackMetadata {
                id: item["id"].as_str().unwrap_or("").to_string(),
                title: item["name"].as_str().unwrap_or("").to_string(),
                artists: if artists.is_empty() { vec![artist_name.clone()] } else { artists },
                album_artist: Some(artist_name.clone()),
                album: album_name.clone(),
                release_date: release_date.clone(),
                genre: album_genre.clone(),
                track_number: (idx + 1) as u32,
                disc_number: item["disc_number"].as_u64().unwrap_or(1) as u32,
                duration_ms: item["duration_ms"].as_u64().unwrap_or(0),
                isrc: None,
                cover_url: cover_url.clone(),
                spotify_url: format!("https://open.spotify.com/track/{}", item["id"].as_str().unwrap_or("")),
            });
        }

        Ok(FetchedCollection {
            id: id.to_string(),
            title: album_name,
            subtitle: artist_name,
            cover_url,
            tracks,
            collection_type: CollectionType::Album,
        })
    }

    async fn fetch_playlist_api(&self, id: &str, token: &str) -> Result<FetchedCollection> {
        let url = format!("https://api.spotify.com/v1/playlists/{}", id);
        let resp_res = self.http.get(&url).bearer_auth(token).send().await?;
        if !resp_res.status().is_success() {
            bail!("Spotify API returned status {}", resp_res.status());
        }
        let resp: Value = resp_res.json().await?;
        if let Some(err) = resp.get("error") {
            bail!("Spotify API error: {:?}", err);
        }

        let playlist_name = resp["name"].as_str().unwrap_or("Playlist").to_string();
        let owner = resp["owner"]["display_name"].as_str().unwrap_or("Spotify").to_string();
        let cover_url = resp["images"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|i| i["url"].as_str())
            .map(str::to_string);

        let mut tracks = Vec::new();
        if let Some(items) = resp["tracks"]["items"].as_array() {
            for (idx, item) in items.iter().enumerate() {
                if let Some(track_item) = item.get("track") {
                    if let Ok(mut t) = parse_api_track(track_item) {
                        t.track_number = (idx + 1) as u32;
                        tracks.push(t);
                    }
                }
            }
        }

        if tracks.is_empty() {
            bail!("No tracks returned from Spotify API for playlist {}", id);
        }

        Ok(FetchedCollection {
            id: id.to_string(),
            title: playlist_name,
            subtitle: format!("By {}", owner),
            cover_url,
            tracks,
            collection_type: CollectionType::Playlist,
        })
    }

    // Public / Unauthenticated methods
    async fn fetch_track_public(&self, id: &str) -> Result<FetchedCollection> {
        // 1. Try Spotify Embed page first (instant, full metadata)
        let embed_url = format!("https://open.spotify.com/embed/track/{}", id);
        if let Ok(resp) = self.http.get(&embed_url).send().await {
            if let Ok(html) = resp.text().await {
                if let Ok(coll) = self.parse_embed_html(id, &html, CollectionType::Track) {
                    if !coll.tracks.is_empty() {
                        return Ok(coll);
                    }
                }
            }
        }

        // 2. Try SongLink API
        let song_url = format!("https://open.spotify.com/track/{}", id);
        let api_url = format!(
            "https://api.song.link/v1-alpha.1/links?url={}&userCountry=US",
            urlencoding::encode(&song_url)
        );

        if let Ok(resp) = self.http.get(&api_url).send().await {
            if let Ok(data) = resp.json::<Value>().await {
                if let Some(entities) = data.get("entitiesByUniqueId") {
                    let spotify_entity_key = format!("SPOTIFY_SONG::{}", id);
                    if let Some(entity) = entities.get(&spotify_entity_key) {
                        let title = entity["title"].as_str().unwrap_or("Unknown Title").to_string();
                        let artist = entity["artistName"].as_str().unwrap_or("Unknown Artist").to_string();
                        let cover_url = entity["thumbnailUrl"].as_str().map(str::to_string);

                        let track = TrackMetadata {
                            id: id.to_string(),
                            title: title.clone(),
                            artists: vec![artist.clone()],
                            album_artist: Some(artist.clone()),
                            album: "Single".to_string(),
                            release_date: String::new(),
                            genre: None,
                            track_number: 1,
                            disc_number: 1,
                            duration_ms: 0,
                            isrc: None,
                            cover_url: cover_url.clone(),
                            spotify_url: song_url,
                        };

                        return Ok(FetchedCollection {
                            id: id.to_string(),
                            title,
                            subtitle: artist,
                            cover_url,
                            tracks: vec![track],
                            collection_type: CollectionType::Track,
                        });
                    }
                }
            }
        }

        // 3. Fallback: Spotify Embed oEmbed endpoint
        let oembed_url = format!(
            "https://open.spotify.com/oembed?url=https://open.spotify.com/track/{}",
            id
        );
        let resp: Value = self.http.get(&oembed_url).send().await?.json().await?;

        let title = resp["title"].as_str().unwrap_or("Unknown Title").to_string();
        let cover_url = resp["thumbnail_url"].as_str().map(str::to_string);

        let track = TrackMetadata {
            id: id.to_string(),
            title: title.clone(),
            artists: vec!["Spotify Artist".to_string()],
            album_artist: None,
            album: "Unknown Album".to_string(),
            release_date: String::new(),
            genre: None,
            track_number: 1,
            disc_number: 1,
            duration_ms: 0,
            isrc: None,
            cover_url: cover_url.clone(),
            spotify_url: format!("https://open.spotify.com/track/{}", id),
        };

        Ok(FetchedCollection {
            id: id.to_string(),
            title,
            subtitle: "Spotify Track".to_string(),
            cover_url,
            tracks: vec![track],
            collection_type: CollectionType::Track,
        })
    }

    async fn fetch_album_public(&self, id: &str) -> Result<FetchedCollection> {
        let embed_url = format!("https://open.spotify.com/embed/album/{}", id);
        let html = self.http.get(&embed_url).send().await?.text().await?;

        let mut coll = self.parse_embed_html(id, &html, CollectionType::Album)?;

        // If cover art was missing from embed JSON, fetch from oEmbed fallback
        if coll.cover_url.is_none() {
            let oembed_url = format!("https://open.spotify.com/oembed?url=https://open.spotify.com/album/{}", id);
            if let Ok(resp) = self.http.get(&oembed_url).send().await {
                if let Ok(val) = resp.json::<Value>().await {
                    if let Some(thumb) = val["thumbnail_url"].as_str() {
                        let thumb_str = thumb.to_string();
                        coll.cover_url = Some(thumb_str.clone());
                        for t in &mut coll.tracks {
                            if t.cover_url.is_none() {
                                t.cover_url = Some(thumb_str.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(coll)
    }

    async fn fetch_playlist_public(&self, id: &str) -> Result<FetchedCollection> {
        let embed_url = format!("https://open.spotify.com/embed/playlist/{}", id);
        let html = self.http.get(&embed_url).send().await?.text().await?;

        let mut coll = self.parse_embed_html(id, &html, CollectionType::Playlist)?;

        if coll.cover_url.is_none() {
            let oembed_url = format!("https://open.spotify.com/oembed?url=https://open.spotify.com/playlist/{}", id);
            if let Ok(resp) = self.http.get(&oembed_url).send().await {
                if let Ok(val) = resp.json::<Value>().await {
                    if let Some(thumb) = val["thumbnail_url"].as_str() {
                        let thumb_str = thumb.to_string();
                        coll.cover_url = Some(thumb_str.clone());
                        for t in &mut coll.tracks {
                            if t.cover_url.is_none() {
                                t.cover_url = Some(thumb_str.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(coll)
    }

    fn parse_embed_html(&self, id: &str, html: &str, coll_type: CollectionType) -> Result<FetchedCollection> {
        // Search for <script id="__NEXT_DATA__" type="application/json">...</script>
        let re = Regex::new(r#"<script id="__NEXT_DATA__"[^>]*>(.*?)</script>"#)
            .unwrap();

        if let Some(caps) = re.captures(html) {
            if let Some(json_str) = caps.get(1) {
                if let Ok(data) = serde_json::from_str::<Value>(json_str.as_str()) {
                    let entity_opt = data.pointer("/props/pageProps/state/data/entity")
                        .or_else(|| data.pointer("/props/pageProps/data/entity"))
                        .or_else(|| data.pointer("/props/pageProps/entity"));

                    if let Some(entity) = entity_opt {
                        let title = entity["name"].as_str()
                            .or_else(|| entity["title"].as_str())
                            .unwrap_or("Collection")
                            .to_string();

                        let artist_name = entity["artists"]
                            .as_array()
                            .and_then(|a| a.first())
                            .and_then(|art| art["name"].as_str())
                            .or_else(|| entity["subtitle"].as_str())
                            .unwrap_or("Spotify")
                            .to_string();

                        let subtitle = if coll_type == CollectionType::Playlist {
                            entity["subtitle"].as_str().unwrap_or("Spotify Playlist").to_string()
                        } else {
                            artist_name.clone()
                        };

                        let album_artist = if coll_type == CollectionType::Album {
                            Some(artist_name.clone())
                        } else {
                            None
                        };

                        // 1. Cover Art: visualIdentity.image (prefer high resolution), coverArt.sources, images
                        let mut cover_url = entity["visualIdentity"]["image"]
                            .as_array()
                            .and_then(|a| {
                                a.iter().max_by_key(|img| img["maxWidth"].as_u64().unwrap_or(0))
                            })
                            .and_then(|s| s["url"].as_str())
                            .map(str::to_string);

                        if cover_url.is_none() {
                            cover_url = entity["coverArt"]["sources"]
                                .as_array()
                                .and_then(|a| a.last().or_else(|| a.first()))
                                .and_then(|s| s["url"].as_str())
                                .map(str::to_string);
                        }

                        if cover_url.is_none() {
                            cover_url = entity["images"]
                                .as_array()
                                .and_then(|a| a.first())
                                .and_then(|s| s["url"].as_str())
                                .map(str::to_string);
                        }

                        let album_release_date = entity["releaseDate"]["isoString"]
                            .as_str()
                            .or_else(|| entity["releaseDate"].as_str())
                            .unwrap_or("")
                            .to_string();

                        if coll_type == CollectionType::Track {
                            let duration_ms = entity["duration"].as_u64().unwrap_or(0);
                            let release_date = album_release_date.clone();
                            let track_artists: Vec<String> = entity["artists"]
                                .as_array()
                                .unwrap_or(&vec![])
                                .iter()
                                .filter_map(|a| a["name"].as_str().map(str::to_string))
                                .collect();
                            let track_artists = if track_artists.is_empty() { vec![artist_name.clone()] } else { track_artists };

                            let track = TrackMetadata {
                                id: id.to_string(),
                                title: title.clone(),
                                artists: track_artists,
                                album_artist: Some(artist_name.clone()),
                                album: "Single".to_string(),
                                release_date,
                                genre: None,
                                track_number: 1,
                                disc_number: 1,
                                duration_ms,
                                isrc: None,
                                cover_url: cover_url.clone(),
                                spotify_url: format!("https://open.spotify.com/track/{}", id),
                            };

                            return Ok(FetchedCollection {
                                id: id.to_string(),
                                title,
                                subtitle: artist_name,
                                cover_url,
                                tracks: vec![track],
                                collection_type: CollectionType::Track,
                            });
                        }

                        let mut tracks = Vec::new();
                        if let Some(track_list) = entity["trackList"].as_array() {
                            for (idx, item) in track_list.iter().enumerate() {
                                let track_id = item["uri"]
                                    .as_str()
                                    .and_then(|u| u.split(':').last())
                                    .unwrap_or("")
                                    .to_string();
                                let track_title = item["title"].as_str().unwrap_or("Unknown").to_string();
                                let track_subtitle = item["subtitle"].as_str().unwrap_or("Unknown").to_string();
                                let duration_ms = item["duration"].as_u64().unwrap_or(0);

                                // Split track subtitle into distinct artists
                                let artists: Vec<String> = track_subtitle
                                    .split(',')
                                    .map(|s| s.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect();
                                let artists = if artists.is_empty() { vec![track_subtitle] } else { artists };

                                tracks.push(TrackMetadata {
                                    id: track_id.clone(),
                                    title: track_title,
                                    artists,
                                    album_artist: album_artist.clone(),
                                    album: title.clone(),
                                    release_date: album_release_date.clone(),
                                    genre: None,
                                    track_number: (idx + 1) as u32,
                                    disc_number: 1,
                                    duration_ms,
                                    isrc: None,
                                    cover_url: cover_url.clone(),
                                    spotify_url: format!("https://open.spotify.com/track/{}", track_id),
                                });
                            }
                        }

                        return Ok(FetchedCollection {
                            id: id.to_string(),
                            title,
                            subtitle,
                            cover_url,
                            tracks,
                            collection_type: coll_type,
                        });
                    }
                }
            }
        }

        bail!("Could not extract track collection from Spotify embed data")
    }

    /// Fetches image binary data for album cover embedding.
    pub async fn fetch_cover_art(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self.http.get(url).send().await?;
        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Tries to resolve a genre using iTunes Search API given artist and album/title.
    pub async fn fetch_genre_fallback(&self, artist: &str, album_or_title: &str) -> Option<String> {
        let query = format!("{} {}", artist, album_or_title);
        let url = format!(
            "https://itunes.apple.com/search?term={}&entity=song&limit=1",
            urlencoding::encode(&query)
        );

        if let Ok(resp) = self.http.get(&url).send().await {
            if let Ok(val) = resp.json::<Value>().await {
                if let Some(items) = val["results"].as_array() {
                    if let Some(first) = items.first() {
                        if let Some(genre) = first["primaryGenreName"].as_str() {
                            if !genre.trim().is_empty() {
                                return Some(genre.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

fn parse_api_track(item: &Value) -> Result<TrackMetadata> {
    let id = item["id"].as_str().unwrap_or("").to_string();
    let title = item["name"].as_str().unwrap_or("Unknown Title").to_string();
    let artists: Vec<String> = item["artists"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|a| a["name"].as_str().map(str::to_string))
        .collect();

    let album = item["album"]["name"].as_str().unwrap_or("Unknown Album").to_string();
    let album_artist = item["album"]["artists"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|a| a["name"].as_str())
        .map(str::to_string);

    let release_date = item["album"]["release_date"].as_str().unwrap_or("").to_string();
    let track_number = item["track_number"].as_u64().unwrap_or(1) as u32;
    let disc_number = item["disc_number"].as_u64().unwrap_or(1) as u32;
    let duration_ms = item["duration_ms"].as_u64().unwrap_or(0);
    let isrc = item["external_ids"]["isrc"].as_str().map(str::to_string);
    let cover_url = item["album"]["images"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|i| i["url"].as_str())
        .map(str::to_string);

    Ok(TrackMetadata {
        id: id.clone(),
        title,
        artists,
        album_artist,
        album,
        release_date,
        genre: None,
        track_number,
        disc_number,
        duration_ms,
        isrc,
        cover_url,
        spotify_url: format!("https://open.spotify.com/track/{}", id),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_spotify_track_url() {
        let link = parse_spotify_link("https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT?si=abc123");
        assert_eq!(link, Some(SpotifyLink::Track("4cOdK2wGLETKBW3PvgPWqT".to_string())));
    }

    #[test]
    fn test_parse_spotify_intl_url() {
        let link = parse_spotify_link("https://open.spotify.com/intl-de/track/4cOdK2wGLETKBW3PvgPWqT?si=abc123");
        assert_eq!(link, Some(SpotifyLink::Track("4cOdK2wGLETKBW3PvgPWqT".to_string())));

        let album_link = parse_spotify_link("https://open.spotify.com/intl-en/album/1DFixLWuPkv3KT3TnV35m3");
        assert_eq!(album_link, Some(SpotifyLink::Album("1DFixLWuPkv3KT3TnV35m3".to_string())));

        let playlist_link = parse_spotify_link("https://open.spotify.com/intl-es-419/playlist/37i9dQZF1DXcBWIGoYBM5M");
        assert_eq!(playlist_link, Some(SpotifyLink::Playlist("37i9dQZF1DXcBWIGoYBM5M".to_string())));
    }

    #[test]
    fn test_parse_spotify_album_url() {
        let link = parse_spotify_link("https://open.spotify.com/album/1DFixLWuPkv3KT3TnV35m3");
        assert_eq!(link, Some(SpotifyLink::Album("1DFixLWuPkv3KT3TnV35m3".to_string())));
    }

    #[test]
    fn test_parse_spotify_playlist_url() {
        let link = parse_spotify_link("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M");
        assert_eq!(link, Some(SpotifyLink::Playlist("37i9dQZF1DXcBWIGoYBM5M".to_string())));
    }

    #[test]
    fn test_parse_spotify_uri() {
        let link = parse_spotify_link("spotify:track:4cOdK2wGLETKBW3PvgPWqT");
        assert_eq!(link, Some(SpotifyLink::Track("4cOdK2wGLETKBW3PvgPWqT".to_string())));

        let artist = parse_spotify_link("spotify:artist:6s2tKG7vDTgCLpCG4nlIX5");
        assert_eq!(artist, Some(SpotifyLink::Artist("6s2tKG7vDTgCLpCG4nlIX5".to_string())));
    }

    #[test]
    fn test_parse_spotify_various_url_formats() {
        // Embed album
        let embed = parse_spotify_link("https://open.spotify.com/embed/album/1DFixLWuPkv3KT3TnV35m3");
        assert_eq!(embed, Some(SpotifyLink::Album("1DFixLWuPkv3KT3TnV35m3".to_string())));

        // Artist URL
        let artist = parse_spotify_link("https://open.spotify.com/artist/6s2tKG7vDTgCLpCG4nlIX5");
        assert_eq!(artist, Some(SpotifyLink::Artist("6s2tKG7vDTgCLpCG4nlIX5".to_string())));

        // User playlist URL
        let user_pl = parse_spotify_link("https://open.spotify.com/user/spotify/playlist/37i9dQZF1DXcBWIGoYBM5M");
        assert_eq!(user_pl, Some(SpotifyLink::Playlist("37i9dQZF1DXcBWIGoYBM5M".to_string())));

        // URL with quotes and extra copy text
        let wrapped = parse_spotify_link("\"https://open.spotify.com/album/1DFixLWuPkv3KT3TnV35m3?si=test123\"");
        assert_eq!(wrapped, Some(SpotifyLink::Album("1DFixLWuPkv3KT3TnV35m3".to_string())));

        let copy_text = parse_spotify_link("Listen to this album: https://open.spotify.com/album/1DFixLWuPkv3KT3TnV35m3 on Spotify");
        assert_eq!(copy_text, Some(SpotifyLink::Album("1DFixLWuPkv3KT3TnV35m3".to_string())));
    }

    #[tokio::test]
    async fn test_live_fetch_album_public() {
        let auth = Arc::new(AuthManager::new());
        let client = SpotifyClient::new(auth);
        let res = client.fetch_album_public("1DFixLWuPkv3KT3TnV35m3").await;
        assert!(res.is_ok());
        let coll = res.unwrap();
        assert!(!coll.tracks.is_empty(), "Album tracks must not be empty");
        assert_eq!(coll.title, "Emotion (Deluxe)");
        assert_eq!(coll.subtitle, "Carly Rae Jepsen");
        assert_eq!(coll.tracks.len(), 15);
    }

    #[tokio::test]
    async fn test_live_fetch_link_album() {
        let auth = Arc::new(AuthManager::new());
        let client = SpotifyClient::new(auth);
        let link = SpotifyLink::Album("1DFixLWuPkv3KT3TnV35m3".to_string());
        let res = client.fetch_link(&link).await;
        assert!(res.is_ok());
        let coll = res.unwrap();
        assert!(!coll.tracks.is_empty());
        assert_eq!(coll.title, "Emotion (Deluxe)");
        assert_eq!(coll.subtitle, "Carly Rae Jepsen");
    }

    #[tokio::test]
    async fn test_live_fetch_link_track() {
        let auth = Arc::new(AuthManager::new());
        let client = SpotifyClient::new(auth);
        let link = SpotifyLink::Track("4cOdK2wGLETKBW3PvgPWqT".to_string());
        let res = client.fetch_link(&link).await;
        assert!(res.is_ok());
        let coll = res.unwrap();
        assert_eq!(coll.tracks.len(), 1);
        assert_eq!(coll.title, "Never Gonna Give You Up");
        assert_eq!(coll.subtitle, "Rick Astley");
    }

    #[tokio::test]
    async fn test_resolve_spotify_link() {
        let auth = Arc::new(AuthManager::new());
        let client = SpotifyClient::new(auth);
        let link = client.resolve_spotify_link("https://open.spotify.com/album/1DFixLWuPkv3KT3TnV35m3?si=123").await;
        assert_eq!(link, Some(SpotifyLink::Album("1DFixLWuPkv3KT3TnV35m3".to_string())));
    }
}
