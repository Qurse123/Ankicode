# Ankicode

A macOS-first, local-only spaced-repetition desktop app for coding problems.
This repository currently contains the Tauri 2, React/TypeScript, Rust, SQLite,
FSRS, and Chromium extension foundation. Product behavior is implemented in
later tasks.

The approved MVP boundaries are recorded in
[`docs/superpowers/specs/2026-07-18-ankicode-mvp-design.md`](docs/superpowers/specs/2026-07-18-ankicode-mvp-design.md).

## Prerequisites

- macOS
- Node.js and npm
- The stable Rust toolchain installed with `rustup`
- Tauri's macOS system prerequisites, including Xcode Command Line Tools

## Setup

```sh
npm install
npm run tauri dev
```

Run the browser-only React shell with `npm run dev`.

## Checks and builds

```sh
npm run format:check
npm run lint
npm run typecheck
npm test
npm run build
npm run test:extension
npm run build:extension
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

## Load the Chromium extension

1. Run `npm run build:extension`.
2. Open `chrome://extensions` in a Chromium browser.
3. Enable **Developer mode**.
4. Choose **Load unpacked** and select `extension/dist`.

The extension is only a compile-safe MV3 scaffold at this stage. Pairing,
capture, outbox, and loopback API behavior are intentionally not implemented.
