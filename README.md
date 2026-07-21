# Ankicode

Local Anki-for-LeetCode: a macOS-first, local-only spaced-repetition desktop app
for coding problems. Track problems in My List, get a daily assignment, capture
accepted submissions from a Chromium extension over a loopback API, then rate
and reschedule with FSRS — all data stays on this machine.

This repository is a Tauri 2 app (React/TypeScript frontend, Rust/SQLite/FSRS
backend) plus an MV3 Chromium extension. The approved MVP boundaries are in
[`docs/superpowers/specs/2026-07-18-ankicode-mvp-design.md`](docs/superpowers/specs/2026-07-18-ankicode-mvp-design.md).

## Prerequisites

- macOS
- Node.js `^20.19.0` or `>=22.12.0`, and npm
- The stable Rust toolchain installed with `rustup` (`cargo` on `PATH`)
- Tauri's macOS system prerequisites, including Xcode Command Line Tools

## Run the desktop app (no Cursor / no separate backend)

The Tauri `.app` already includes the Rust backend, SQLite, and the loopback
API on `127.0.0.1:17342`. You do **not** need to keep a terminal or Cursor open.

```sh
npm install
npm run tauri build -- --bundles app
cp -R src-tauri/target/release/bundle/macos/Ankicode.app /Applications/
open -a Ankicode
```

If you still see a “DESKTOP SCAFFOLD” screen, you’re opening an old install —
replace `/Applications/Ankicode.app` with a fresh build as above.

### Development (hot reload)

```sh
npm install
npx playwright install chromium
npm run tauri dev
```

`tauri dev` starts Vite + the Rust backend together. Browser-only UI (no
backend): `npm run dev`.

## Extension

1. Run `npm run build:extension`.
2. Open `chrome://extensions` in a Chromium browser.
3. Enable **Developer mode**.
4. Choose **Load unpacked** and select `extension/dist`.
5. Pair with the code shown during onboarding (or later in Settings).

### Loopback API

The desktop app exposes a local HTTP API on **`127.0.0.1:17342`** for the
extension to submit accepted completions. It is intentionally loopback-only.

## Verification

```sh
npm run format:check
npm run lint
npm run typecheck
npm test
npm run test:extension
npm run test:rust
npm run test:e2e
npm run build
npm run build:extension
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run tauri build
```

Or run the aggregated gate:

```sh
npm run check
```

`test:e2e` runs a mocked Playwright journey against the Vite React shell
(Chromium only; no desktop binary required).

## Build notes

`npm run tauri build` produces an **unsigned internal macOS build**. Code
signing, notarization, and cloud sync are out of scope for this MVP.
