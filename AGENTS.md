# transfer_music_favourites Agents Guide

This project is a Rust CLI tool for synchronizing Navidrome favourites to a local directory.

## Conventions

- **Language:** Rust (2021 edition)
- **Formatting:** `cargo fmt`
- **Linting:** `cargo clippy -- -D warnings`
- **Testing:** `cargo test`
- **Building:** `cargo build --release`

## Architecture

The application uses a modular structure to isolate concerns:
- `cli.rs`: Command line arguments definition via `clap`.
- `api.rs`: Subsonic API communication and authentication logic.
- `sync.rs`: Path translation and file system operations (async copying).
- `models.rs`: JSON data models via `serde`.
- `lib.rs`: Library definitions.
- `main.rs`: Entry point orchestration.

## Key Learnings & Rules

1. **Module Configuration:** Modules (like `api` and `sync`) should have their own dedicated configuration structs (e.g., `ApiConfig`, `SyncConfig`). Do not pass the global `clap::Args` struct directly into library functions. This improves testability and decoupling.
2. **Async File I/O:** When performing file operations inside a tokio runtime, use `tokio::fs` (e.g., `tokio::fs::copy`, `tokio::fs::create_dir_all`) instead of `std::fs` to prevent blocking the async executor.
3. **HTTP Errors:** Always use `.error_for_status()` when making requests with `reqwest` before parsing the response body. This prevents the application from trying to parse HTML error pages (like 404s) as JSON.
4. **Path Prefixes:** When stripping directory prefixes, use `std::path::Path::strip_prefix` rather than string `starts_with` checks to avoid false positives on similar directory names.
5. **Async Process Execution:** When executing external commands (like `ffmpeg`), use `tokio::process::Command` instead of `std::process::Command` to prevent blocking the async executor while waiting for the child process to complete.
6. **Async I/O Concurrency:** Use `futures::stream::StreamExt` and `for_each_concurrent` for parallelizing I/O-bound tasks. Do not use CPU-bound parallelism libraries like `rayon` inside an async context. Use `std::thread::available_parallelism()` to dynamically scale workers based on available cores.
7. **FFmpeg Audio Conversion:** When converting audio, ensure temporary output files retain the target extension (e.g. `.tmp.song.opus`, not `song.opus.tmp`) so FFmpeg correctly infers the container format. Always pass `-vn` to drop video streams (like embedded album art) which can cause audio muxers to fail.
