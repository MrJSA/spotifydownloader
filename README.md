# 🎵 Spotify Downloader

[![Platform: Windows | macOS | Linux](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-0078D6?style=flat-square)](https://github.com/MrJSA/spotifydownloader)
[![Language: Rust](https://img.shields.io/badge/Language-Rust%202021-DEA584?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![GUI: egui / eframe](https://img.shields.io/badge/GUI-egui%20%2F%20eframe-1ed760?style=flat-square)](https://github.com/emilk/egui)
[![Audio: librespot](https://img.shields.io/badge/Audio-librespot-1DB954?style=flat-square&logo=spotify)](https://github.com/librespot-org/librespot)
[![Build Tool: Cargo](https://img.shields.io/badge/Build%20Tool-Cargo-DEA584?style=flat-square&logo=cargo)](https://crates.io)
[![License: MIT](https://img.shields.io/badge/License-MIT-green?style=flat-square)](LICENSE)

A high-performance, modern cross-platform desktop application to search, browse, and download high-fidelity music from Spotify. Features a sleek, Fastpotify-inspired dark UI, dual download pipelines (direct Spotify CDN 320 kbps stream for Premium and multi-source fallback), rich embedded metadata, and automatic library organization—all packaged in a **zero-dependency, standalone application** using Rust and egui.

---

## 📦 How to Install & Run

1. Head over to the **[Releases](https://github.com/MrJSA/spotifydownloader/releases)** page.
2. Download the package for your operating system:
   * **Windows:** Download `SpotifyDownloader-Windows.exe` and run it directly.
   * **macOS:** Download `SpotifyDownloader-macOS.dmg`, double-click to mount, and drag the app to your `Applications` folder.
   * **Linux:** Download `SpotifyDownloader-Linux.tar.gz`, extract, and run `SpotifyDownloader`.

---

<p align="center">
  <img alt="Spotify Downloader Screenshot" src=".github/screenshot1.png" width="90%">
</p>

---

## ✨ Key Features

| Feature | Description |
| :--- | :--- |
| **🎨 Fastpotify-Inspired UI** | Sleek, dark Spotify aesthetic (`#1ed760` accent) built with `egui` and `eframe` for 0 web bloat, instant launch, and 60 FPS responsiveness. |
| **⚡ Dual Download Pipeline** | Direct 320 kbps Vorbis decryption from Spotify Access Points (Premium) with automatic high-bitrate multi-source resolver fallback (Free & unauthenticated). |
| **🎧 Audiophile Quality Formats** | Complete format freedom: **FLAC** (Lossless), **M4A (ALAC Lossless)**, **WAV** (16-bit PCM), **AIFF** (Apple PCM), **M4A (256 kbps AAC)**, and **MP3** (320, 192, and 128 kbps). |
| **🔍 In-App Catalog Browsing** | Search artists, albums, and tracks directly inside the app with 1-click album downloads and artist discography navigation. |
| **🔗 Universal Link Support** | Paste any Spotify URL (track, album, playlist, artist, localized or mobile shortlinks) for immediate retrieval. |
| **🏷️ Rich Metadata & Cover Art** | Automatically embeds album artwork (`covr`, `APIC`, `METADATA_BLOCK_PICTURE`), tags (title, artist, album, year, track number), and saves `cover.jpg`. |
| **📂 Smart Organization** | Automatically sorts music into `{Artist}/{Album}/{TrackNumber} - {Title}.{ext}` with clean sanitized paths and zero stray temp files. |
| **💾 Persistent Settings** | Automatically saves your download folder, preferred audio format, and Spotify login state across app restarts. |
| **🔌 Standalone Executables** | Pre-built standalone native releases for Windows (`.exe`), macOS (`.dmg`), and Linux (`.tar.gz`). |

---

## 🛠 How It Works

The application combines a high-performance egui GUI with an asynchronous Rust audio engine and dual-stream pipeline:

```mermaid
graph TD
    A[Spotify Downloader] -->|1. Runs egui/eframe GUI| B(App Window)
    B -->|2. Search query or Spotify Link| C{Input Type}
    C -->|Search text| D[Spotify Web Catalog API]
    C -->|Spotify URL| E[Spotify Link Resolver]
    D --> F[Albums, Artists & Tracks Shelf]
    E --> F
    F -->|3. Click Download| G{Spotify Account Status}
    G -->|Premium Credentials| H[Direct Spotify CDN Stream Decryptor - librespot]
    G -->|Free / Unauthenticated| I[High-Bitrate Multi-Source Resolver]
    H -->|320 kbps Vorbis Stream| J[Audio Transcoder & Normalizer]
    I -->|High-Quality Audio Stream| J
    J -->|FLAC / M4A / MP3| K[Tag & Metadata Embedder]
    K -->|Auto Organize| L[Artist / Album / Track.ext]
```

---

## 💻 Developer Guide

If you'd like to clone the repository and build from source, follow this workflow:

### Workspace Pre-requisites
1. **Rust Toolchain**: Rust 1.80+ (Install via [rustup.rs](https://rustup.rs))
2. **FFmpeg**: Required for audio transcoding and stream extraction:
   * **Windows**: `winget install Gyan.FFmpeg` or `scoop install ffmpeg`
   * **macOS**: `brew install ffmpeg`
   * **Linux**: `sudo apt install ffmpeg libasound2-dev libx11-dev libgl1-mesa-dev libxkbcommon-dev libwayland-dev`

### Local Development Cycle
* **Run in Development**:
  ```bash
  cargo run
  ```
* **Run Test Suite**:
  ```bash
  cargo test
  ```
* **Build Standalone Release**:
  ```bash
  cargo build --release
  ```
  The compiled executable will be located in `target/release/`.

---

## ⚖️ Disclaimer

This software is developed strictly for educational, personal, and archival purposes. Please respect copyright laws and the terms of service of any media platforms you interact with.

---

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.