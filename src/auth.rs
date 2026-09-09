use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use librespot_core::authentication::Credentials;
use librespot_core::config::SessionConfig;
use librespot_core::session::Session;

pub const DEFAULT_CLIENT_ID: &str = "d420a117a32841c2b3474932e49fb54b"; // Public client ID for desktop/web players
pub const REDIRECT_PORT: u16 = 8989;
pub const REDIRECT_URI: &str = "http://127.0.0.1:8989/login";

pub const SCOPES: &[&str] = &[
    "user-read-private",
    "user-library-read",
    "playlist-read-private",
    "playlist-read-collaborative",
    "user-top-read",
    "user-read-recently-played",
    "user-follow-read",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub product: Option<String>, // "premium" or "free"
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedAuth {
    pub profile: Option<UserProfile>,
    pub tokens: Option<AuthTokens>,
    pub direct_creds: Option<(String, String)>,
}

pub fn auth_config_path() -> PathBuf {
    let dir = directories::ProjectDirs::from("com", "spotifydownloader", "SpotifyDownloader")
        .map(|p| p.config_dir().to_path_buf())
        .or_else(|| directories::UserDirs::new().map(|u| u.home_dir().join(".spotifydownloader")))
        .unwrap_or_else(|| PathBuf::from("config"));
    let _ = std::fs::create_dir_all(&dir);
    dir.join("auth.json")
}

#[derive(Clone)]
pub struct AuthManager {
    pub profile: Arc<Mutex<Option<UserProfile>>>,
    pub tokens: Arc<Mutex<Option<AuthTokens>>>,
    pub is_logging_in: Arc<Mutex<bool>>,
    pub last_error: Arc<Mutex<Option<String>>>,
    pub session: Arc<tokio::sync::Mutex<Option<Session>>>,
    client: reqwest::Client,
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            profile: Arc::new(Mutex::new(None)),
            tokens: Arc::new(Mutex::new(None)),
            is_logging_in: Arc::new(Mutex::new(false)),
            last_error: Arc::new(Mutex::new(None)),
            session: Arc::new(tokio::sync::Mutex::new(None)),
            client: reqwest::Client::new(),
        }
    }

    pub fn load_from_disk_raw() -> Option<PersistedAuth> {
        let path = auth_config_path();
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<PersistedAuth>(&content).ok()
    }

    /// Synchronously loads cached profile and tokens so UI can show user status immediately on startup.
    pub fn load_persisted_cached(&self) {
        if let Some(auth_data) = Self::load_from_disk_raw() {
            if let Some(prof) = auth_data.profile {
                if let Ok(mut p_guard) = self.profile.lock() {
                    *p_guard = Some(prof);
                }
            }
            if let Some(tok) = auth_data.tokens {
                if let Ok(mut t_guard) = self.tokens.lock() {
                    *t_guard = Some(tok);
                }
            }
        }
    }

    /// Asynchronously validates / refreshes tokens, refreshes profile, and reconnects session.
    pub async fn load_persisted(&self) {
        let Some(auth_data) = Self::load_from_disk_raw() else {
            return;
        };

        if let Some(prof) = auth_data.profile {
            if let Ok(mut p_guard) = self.profile.lock() {
                *p_guard = Some(prof);
            }
        }

        if let Some(mut tok) = auth_data.tokens {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if now + 60 >= tok.expires_at {
                if let Some(ref r_tok) = tok.refresh_token {
                    if let Ok(new_tokens) = self.refresh_access_token(r_tok).await {
                        tok = new_tokens;
                    }
                }
            }

            if let Ok(mut t_guard) = self.tokens.lock() {
                *t_guard = Some(tok);
            }

            let _ = self.fetch_user_profile().await;
            self.init_librespot_session().await;
            self.save_to_disk(None);
            return;
        }

        if let Some((user, pass)) = auth_data.direct_creds {
            let _ = self.login_with_credentials(&user, &pass).await;
        }
    }

    pub async fn refresh_access_token(&self, refresh_tok: &str) -> Result<AuthTokens> {
        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", DEFAULT_CLIENT_ID),
            ("refresh_token", refresh_tok),
        ];

        let resp = self.client.post("https://accounts.spotify.com/api/token")
            .form(&params)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            bail!("Token refresh failed: {}", err);
        }

        #[derive(Deserialize)]
        struct RefreshResp {
            access_token: String,
            refresh_token: Option<String>,
            expires_in: u64,
        }

        let body: RefreshResp = resp.json().await?;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();

        let new_tokens = AuthTokens {
            access_token: body.access_token,
            refresh_token: body.refresh_token.or_else(|| Some(refresh_tok.to_string())),
            expires_at: now + body.expires_in,
        };

        if let Ok(mut tok) = self.tokens.lock() {
            *tok = Some(new_tokens.clone());
        }

        Ok(new_tokens)
    }

    pub fn save_to_disk(&self, direct_creds: Option<(String, String)>) {
        let profile = self.profile.lock().ok().and_then(|p| p.clone());
        let tokens = self.tokens.lock().ok().and_then(|t| t.clone());
        let existing_creds = direct_creds.or_else(|| {
            Self::load_from_disk_raw().and_then(|a| a.direct_creds)
        });

        let data = PersistedAuth {
            profile,
            tokens,
            direct_creds: existing_creds,
        };

        let path = auth_config_path();
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(&path, json);
        }
    }

    pub fn is_logged_in(&self) -> bool {
        self.profile.lock().map(|p| p.is_some()).unwrap_or(false)
    }

    pub fn is_premium(&self) -> bool {
        self.profile.lock().ok()
            .and_then(|p| p.as_ref().map(|prof| prof.product.as_deref() == Some("premium")))
            .unwrap_or(false)
    }

    pub fn get_user_name(&self) -> Option<String> {
        self.profile.lock().ok()?.as_ref().map(|p| {
            p.display_name.clone().unwrap_or_else(|| p.id.clone())
        })
    }

    pub fn get_access_token(&self) -> Option<String> {
        self.tokens.lock().ok()?.as_ref().map(|t| t.access_token.clone())
    }

    pub fn get_last_error(&self) -> Option<String> {
        self.last_error.lock().ok()?.clone()
    }

    pub async fn get_session(&self) -> Option<Session> {
        self.session.lock().await.clone()
    }

    pub async fn logout(&self) {
        if let Ok(mut prof) = self.profile.lock() {
            *prof = None;
        }
        if let Ok(mut tok) = self.tokens.lock() {
            *tok = None;
        }
        if let Ok(mut err) = self.last_error.lock() {
            *err = None;
        }
        let mut s_guard = self.session.lock().await;
        *s_guard = None;

        let path = auth_config_path();
        let _ = std::fs::remove_file(path);
    }

    /// Connects a direct librespot session using Spotify username and password.
    pub async fn login_with_credentials(&self, username: &str, password: &str) -> Result<()> {
        if let Ok(mut err_guard) = self.last_error.lock() {
            *err_guard = None;
        }

        let creds = Credentials::with_password(username, password);
        let session_config = SessionConfig {
            device_id: "spotifydownloader_desktop".to_string(),
            ..Default::default()
        };

        let session = Session::new(session_config, None);
        if let Err(e) = session.connect(creds, true).await {
            let msg = format!("Direct Spotify login failed: {:?}", e);
            if let Ok(mut err_guard) = self.last_error.lock() {
                *err_guard = Some(msg.clone());
            }
            bail!(msg);
        }

        let user_id = session.username().to_string();
        let prof = UserProfile {
            id: user_id.clone(),
            display_name: Some(user_id),
            email: None,
            product: Some("premium".to_string()),
            avatar_url: None,
        };

        if let Ok(mut p_guard) = self.profile.lock() {
            *p_guard = Some(prof);
        }

        let mut s_guard = self.session.lock().await;
        *s_guard = Some(session);

        self.save_to_disk(Some((username.to_string(), password.to_string())));

        Ok(())
    }

    /// Initializes a librespot session using the current OAuth access token if user is Premium.
    pub async fn init_librespot_session(&self) {
        if !self.is_premium() {
            return;
        }

        let Some(token) = self.get_access_token() else {
            return;
        };

        let creds = Credentials::with_access_token(&token);
        let session_config = SessionConfig {
            device_id: "spotifydownloader_desktop".to_string(),
            ..Default::default()
        };

        let session = Session::new(session_config, None);
        if let Ok(()) = session.connect(creds, true).await {
            let mut s_guard = self.session.lock().await;
            *s_guard = Some(session);
        }
    }

    /// Initiates PKCE OAuth in the system browser and starts a loopback listener.
    pub async fn start_login_flow(&self) -> Result<()> {
        {
            let mut logging_in = self.is_logging_in.lock().unwrap();
            if *logging_in {
                return Ok(());
            }
            *logging_in = true;
        }

        if let Ok(mut err_guard) = self.last_error.lock() {
            *err_guard = None;
        }

        let res = self.do_login_flow().await;

        {
            let mut logging_in = self.is_logging_in.lock().unwrap();
            *logging_in = false;
        }

        if let Err(ref e) = res {
            if let Ok(mut err_guard) = self.last_error.lock() {
                *err_guard = Some(e.to_string());
            }
        }

        res
    }

    async fn do_login_flow(&self) -> Result<()> {
        // Generate PKCE code verifier and challenge
        let mut random_bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut random_bytes);
        let code_verifier = URL_SAFE_NO_PAD.encode(random_bytes);

        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let code_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

        let mut state_bytes = [0u8; 16];
        rand::rng().fill_bytes(&mut state_bytes);
        let expected_state = URL_SAFE_NO_PAD.encode(state_bytes);

        let auth_url = format!(
            "https://accounts.spotify.com/authorize?client_id={}&response_type=code&redirect_uri={}&code_challenge_method=S256&code_challenge={}&state={}&scope={}",
            DEFAULT_CLIENT_ID,
            urlencoding::encode(REDIRECT_URI),
            code_challenge,
            expected_state,
            urlencoding::encode(&SCOPES.join(" "))
        );

        // Bind loopback listener FIRST before opening browser
        let addr: SocketAddr = format!("127.0.0.1:{}", REDIRECT_PORT).parse()?;
        let listener = TcpListener::bind(addr).await
            .with_context(|| format!("Could not bind loopback login port {}. Is another instance running?", REDIRECT_PORT))?;

        // Open browser
        if let Err(err) = open::that(&auth_url) {
            eprintln!("Failed to open system browser: {}", err);
        }

        // Wait for redirect in a loop with 5-minute timeout
        let deadline = tokio::time::sleep(Duration::from_secs(300));
        tokio::pin!(deadline);

        let code = loop {
            let (mut socket, _) = tokio::select! {
                accepted = listener.accept() => accepted.context("Redirect listener failed")?,
                _ = &mut deadline => bail!("Login timed out after 5 minutes; please try again."),
            };

            let (read, mut write) = socket.split();
            let mut reader = BufReader::new(read);
            let mut request_line = String::new();
            if reader.read_line(&mut request_line).await.is_err() || request_line.is_empty() {
                continue;
            }

            // Check request line: e.g. "GET /login?code=...&state=... HTTP/1.1"
            let target = match request_line.split_whitespace().nth(1) {
                Some(t) => t,
                None => continue,
            };

            let (path, query) = target.split_once('?').unwrap_or((target, ""));
            if path != "/login" {
                // Favicon or stray browser request: return 404 and keep waiting
                let not_found = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = write.write_all(not_found.as_bytes()).await;
                continue;
            }

            let mut code_val = None;
            let mut state_val = None;
            let mut error_val = None;

            for param in query.split('&') {
                if let Some((k, v)) = param.split_once('=') {
                    let decoded = urlencoding::decode(v).map(|s| s.into_owned()).unwrap_or_else(|_| v.to_string());
                    match k {
                        "code" => code_val = Some(decoded),
                        "state" => state_val = Some(decoded),
                        "error" => error_val = Some(decoded),
                        _ => {}
                    }
                }
            }

            if let Some(err) = error_val {
                let err_page = format!(
                    "<!DOCTYPE html><html><body style=\"font-family:sans-serif;background:#15181c;color:#e74c3c;display:flex;justify-content:center;align-items:center;height:80vh;\"><div style=\"text-align:center;\"><h1>Login Refused</h1><p style=\"color:#f2f4f6;\">Spotify returned: {}</p></div></body></html>",
                    err
                );
                let resp = format!(
                    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                    err_page.len(),
                    err_page
                );
                let _ = write.write_all(resp.as_bytes()).await;
                let _ = write.shutdown().await;
                bail!("Spotify sign-in was refused: {}", err);
            }

            if state_val.as_deref() != Some(&expected_state) {
                let err_page = "<!DOCTYPE html><html><body style=\"font-family:sans-serif;background:#15181c;color:#e74c3c;display:flex;justify-content:center;align-items:center;height:80vh;\"><div style=\"text-align:center;\"><h1>State Mismatch</h1><p style=\"color:#f2f4f6;\">Security state check failed. Please try again.</p></div></body></html>";
                let resp = format!(
                    "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                    err_page.len(),
                    err_page
                );
                let _ = write.write_all(resp.as_bytes()).await;
                let _ = write.shutdown().await;
                continue;
            }

            if let Some(c) = code_val {
                let success_page = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Spotify Downloader</title></head><body style=\"font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:#0f1114;color:#f2f4f6;display:flex;justify-content:center;align-items:center;height:90vh;margin:0;\"><div style=\"background:#15181c;border:1px solid #282e38;border-radius:12px;padding:40px;text-align:center;max-width:420px;box-shadow:0 8px 24px rgba(0,0,0,0.5);\"><div style=\"width:56px;height:56px;background:#1ed760;border-radius:50%;display:flex;align-items:center;justify-content:center;margin:0 auto 16px;font-size:28px;color:#000;\">&#10003;</div><h1 style=\"color:#1ed760;margin:0 0 8px;font-size:24px;\">Login Successful!</h1><p style=\"color:#9299a4;font-size:14px;margin:0;\">You can now close this tab and return to Spotify Downloader.</p></div></body></html>";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                    success_page.len(),
                    success_page
                );
                let _ = write.write_all(resp.as_bytes()).await;
                let _ = write.shutdown().await;
                break c;
            }
        };

        // Exchange code for tokens
        self.exchange_code(&code, &code_verifier).await?;
        self.fetch_user_profile().await?;
        self.init_librespot_session().await;
        self.save_to_disk(None);

        Ok(())
    }

    async fn exchange_code(&self, code: &str, verifier: &str) -> Result<()> {
        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", DEFAULT_CLIENT_ID),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("code_verifier", verifier),
        ];

        let resp = self.client.post("https://accounts.spotify.com/api/token")
            .form(&params)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct TokenResp {
            access_token: String,
            refresh_token: Option<String>,
            expires_in: u64,
        }

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            bail!("Spotify token exchange error: {}", err);
        }

        let body: TokenResp = resp.json().await?;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();

        if let Ok(mut tok) = self.tokens.lock() {
            *tok = Some(AuthTokens {
                access_token: body.access_token,
                refresh_token: body.refresh_token,
                expires_at: now + body.expires_in,
            });
        }

        Ok(())
    }

    pub async fn fetch_user_profile(&self) -> Result<()> {
        let token = self.get_access_token().ok_or_else(|| anyhow!("Not logged in"))?;

        let resp = self.client.get("https://api.spotify.com/v1/me")
            .bearer_auth(&token)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct ProfileResp {
            id: String,
            display_name: Option<String>,
            email: Option<String>,
            product: Option<String>,
            images: Option<Vec<ImageResp>>,
        }

        #[derive(Deserialize)]
        struct ImageResp {
            url: String,
        }

        if resp.status().is_success() {
            let body: ProfileResp = resp.json().await?;
            let avatar = body.images.and_then(|imgs| imgs.into_iter().next().map(|i| i.url));
            let profile = UserProfile {
                id: body.id,
                display_name: body.display_name,
                email: body.email,
                product: body.product,
                avatar_url: avatar,
            };

            if let Ok(mut prof) = self.profile.lock() {
                *prof = Some(profile);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persisted_auth_round_trip() {
        let auth_data = PersistedAuth {
            profile: Some(UserProfile {
                id: "testuser123".to_string(),
                display_name: Some("Test User".to_string()),
                email: Some("test@example.com".to_string()),
                product: Some("premium".to_string()),
                avatar_url: None,
            }),
            tokens: Some(AuthTokens {
                access_token: "mock_access_token_123".to_string(),
                refresh_token: Some("mock_refresh_token_456".to_string()),
                expires_at: 1700000000,
            }),
            direct_creds: Some(("spotuser".to_string(), "spotpass".to_string())),
        };

        let json = serde_json::to_string(&auth_data).expect("Must serialize");
        let decoded: PersistedAuth = serde_json::from_str(&json).expect("Must deserialize");

        assert_eq!(decoded.profile.as_ref().unwrap().id, "testuser123");
        assert_eq!(decoded.profile.as_ref().unwrap().product.as_deref(), Some("premium"));
        assert_eq!(decoded.tokens.as_ref().unwrap().access_token, "mock_access_token_123");
        assert_eq!(decoded.tokens.as_ref().unwrap().refresh_token.as_deref(), Some("mock_refresh_token_456"));
        assert_eq!(decoded.direct_creds, Some(("spotuser".to_string(), "spotpass".to_string())));
    }
}

