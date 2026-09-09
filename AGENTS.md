# Spotify Downloader Agent Guide

Refer to [`.agent`](file:///c:/Users/joshu/RustroverProjects/spotifydownloader/.agent) for the complete rules and architectural constraints.

Key Directives:
- Pure Rust codebase.
- Native GUI using `egui` and `eframe` inspired by `fastpotify`.
- Formats: MP3 (320k), M4A (256k), FLAC (Lossless).
- Audio pipeline: Direct Spotify stream, live play-and-record simultaneous sink, multi-source fallback.
- Auto folder structure: `Artist/Album/TrackNumber - Title.ext`.
- Rich metadata and embedded cover art.
