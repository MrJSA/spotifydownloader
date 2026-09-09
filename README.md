# 🎵 Spotify Downloader

A modern, native desktop application written entirely in **Rust** for searching, browsing, and downloading high-fidelity music from Spotify. Features a sleek, Spotify-inspired dark UI built with `egui` / `eframe`, dual download pipelines (direct Spotify CDN stream for Premium and multi-source resolver fallback), rich embedded metadata, and automatic library organization.

---

## ✨ Features

- **🎨 Fastpotify-Inspired Native GUI**:
  - Dark Spotify aesthetic (`#0f1114`, `#15181c`, `#1d2127`, `#1ed760`).
  - Native performance with 0 web bloat via `egui` and `eframe`.
  - Left navigation sidebar, responsive top bar with format switcher, central dynamic views, and bottom playback/recording panel.

- **⚡ Dual Audio Download Pipeline**:
  - **Direct Spotify CDN (Spotify Premium)**:
    - Native `librespot` audio engine streaming directly from Spotify Access Points.
    - True 320 kbps Vorbis stream decryption with on-the-fly keymaster authentication.
  - **Smart Multi-Source Fallback (Free & Unauthenticated)**:
    - High-bitrate audio stream extraction from alternative sources (YouTube Music / FFmpeg).
    - Seamless and automatic: downloads always succeed whether signed in or not.

- **🎧 Audio Formats & Bitrates**:
  - **FLAC**: Lossless 16-bit / 44.1 kHz.
  - **M4A**: High-Efficiency AAC (up to 256 kbps).
  - **MP3**: High-Quality MPEG Audio (up to 320 kbps).

- **🔍 In-App Spotify Catalog Search & Browsing**:
  - Search across artists, albums, and tracks directly inside the app.
  - **1-Click Album Downloads**: Download entire albums with a single click directly from the search shelf.
  - Explore artist discographies and view complete tracklists with album artwork.
  - Paste any Spotify URL (Track, Album, or Playlist) directly into the search bar.

- **🏷️ Rich Metadata & Embedded Artwork**:
  - Embedded front album artwork:
    - `ID3v2 APIC` frame for MP3.
    - `METADATA_BLOCK_PICTURE` for FLAC.
    - `covr` atom for M4A.
  - Automatic `cover.jpg` saving in the album folder.
  - Full ID3/Vorbis/MP4 tags: Track Title, Artist, Album, Album Artist, Release Year, Track Number, Total Tracks, and ISRC.

- **📂 Clean Library Organization**:
  - Automatically organized hierarchy:
    ```text
    {Download Directory}/
    └── {Album Artist}/
        └── {Album Title}/
            ├── cover.jpg
            ├── 01 - Track Name.flac
            ├── 02 - Track Name.flac
            └── ...
    ```
  - Cross-platform filename and folder sanitization (`Windows`, `Linux`, `macOS`).
  - Zero stray files: all temporary chunks and opus caches are isolated and automatically purged upon completion.

---

## 🚀 Getting Started

### Prerequisites

1. **Rust**: Rust toolchain (version 1.80+ recommended).
   - Install via [rustup.rs](https://rustup.rs/):
     ```bash
     curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
     ```
2. **FFmpeg**: Required for audio transcoding and stream extraction.
   - **Windows**: Install via `winget install Gyan.FFmpeg` or `scoop install ffmpeg`.
   - **macOS**: Install via `brew install ffmpeg`.
   - **Linux**: Install via `sudo apt install ffmpeg` or your distribution's package manager.

### Building & Running

1. **Clone the repository**:
   ```bash
   git clone https://github.com/MrJSA/spotifydownloader.git
   cd spotifydownloader
   ```

2. **Run in development mode**:
   ```bash
   cargo run
   ```

3. **Build an optimized release binary**:
   ```bash
   cargo build --release
   ```
   The compiled executable will be located in `./target/release/spotifydownloader`.

---

## 📖 Usage Guide

### 1. In-App Searching & Browsing
- Type any artist name, album name, or song title in the top search bar and press **Enter** (or click a quick search chip like *The Weeknd*, *Daft Punk*, etc.).
- **Download an Entire Album**: Click the **"⬇ Download Album"** button on any album card to enqueue all songs automatically.
- **Download a Track**: Click the **"⬇ Download"** button on individual track rows.
- **Paste Spotify Links**: Paste any track, album, or playlist URL directly into the search bar.

### 2. Format Selection
- Use the format pill toggle in the top bar to switch between **FLAC**, **M4A**, or **MP3** at any time.

### 3. Spotify Account Sign-In (Optional)
- Navigate to the **Account** tab via the sidebar or top bar.
- **Spotify Premium Users**: Sign in with your Spotify username and password to activate direct 320 kbps Spotify CDN downloads.
- **Free Users**: You can browse the catalog and download songs without signing in; the app will automatically use the high-quality multi-source resolver.

### 4. Customizing Download Directory & Settings
- Open **Settings** from the sidebar to configure your default download destination folder and default audio quality format.

---

## 🛠️ Architecture

```text
spotifydownloader/
├── src/
│   ├── main.rs                   # App entrypoint and eframe initialization
│   ├── app.rs                    # Central application state & background task coordination
│   ├── model.rs                  # Data models (Track, Album, Artist, DownloadTask, Settings)
│   ├── theme.rs                  # Custom Fastpotify dark palette, styles, & vector painters
│   ├── spotify.rs                # Spotify Web API client & catalog search engine
│   ├── auth.rs                   # Spotify authentication & Librespot session manager
│   ├── downloader/
│   │   ├── mod.rs                # Download queue manager & routing pipeline
│   │   ├── spotify_stream.rs     # Direct Spotify CDN stream decryptor (librespot)
│   │   └── resolver.rs           # Multi-source resolver & FFmpeg transcoder
│   └── ui/
│       ├── mod.rs                # UI router and event handling
│       ├── topbar.rs             # Search bar, format pills, and user avatar
│       ├── sidebar.rs            # Navigation menu
│       ├── search_view.rs        # In-app catalog browser (Albums, Artists, Tracks)
│       ├── queue_view.rs         # Active download queue & progress bars
│       ├── history_view.rs       # Completed downloads history
│       ├── settings_view.rs      # Directory picker and audio preferences
│       ├── account_view.rs       # Spotify login & subscription status
│       └── player_bar.rs         # Bottom playback & live recording bar
└── Cargo.toml                    # Dependencies and patch configurations
```

---

## ⚖️ Disclaimer

This software is developed strictly for educational, personal, and archival purposes. Please respect copyright laws and the terms of service of any media platforms you interact with.

---

## 📄 License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.