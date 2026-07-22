# Ankicode

Ankicode is local Anki-style spaced repetition for LeetCode.
Rate problems with Again / Hard / Medium / Easy (FSRS), get a daily queue, and keep every review on your Mac—no cloud sync.

This repository is a Tauri 2 desktop app (React/TypeScript + Rust/SQLite/FSRS) plus a Chromium MV3 extension. Design boundaries live in [`docs/superpowers/specs/2026-07-18-ankicode-mvp-design.md`](docs/superpowers/specs/2026-07-18-ankicode-mvp-design.md).

## Prerequisites

- **macOS** (desktop app is macOS-first)
- Node.js `^20.19.0` or `>=22.12.0`, and npm
- Stable Rust via `rustup` (`cargo` on `PATH`)
- Xcode Command Line Tools (Tauri macOS deps)

## Run on your computer

### Option A — installed desktop app (recommended)

Builds a standalone `.app` that includes the Rust backend, SQLite, and the loopback API on `127.0.0.1:17342`. No terminal needs to stay open.

```sh
git clone https://github.com/Qurse123/Ankicode.git
cd Ankicode
npm install
npm run tauri -- build --bundles app
```

Then install and open (path may be `src-tauri/target/...` or your `CARGO_TARGET_DIR`):

```sh
# Typical local build output:
cp -R src-tauri/target/release/bundle/macos/Ankicode.app /Applications/
open -a Ankicode
```

If the UI looks like an old scaffold, replace `/Applications/Ankicode.app` with a fresh build and reopen.

### Option B — development (hot reload)

```sh
npm install
npx playwright install chromium
npm run tauri -- dev
```

`tauri dev` starts Vite and the Rust backend together. Frontend-only (no backend): `npm run dev`.

### Chromium extension (Accepted → rate prompt)

Keep the desktop app running, then:

1. `npm run build:extension`
2. Open `chrome://extensions` → enable **Developer mode**
3. **Load unpacked** → select `extension/dist` (not the repo root `dist/`)
4. Pair once with the code from Ankicode **Settings** (or onboarding). The extension
   stores a long-lived token—you should not need to re-pair unless you Unpair or
   reload the extension from a new path.

The extension talks only to the local loopback API at **`http://127.0.0.1:17342`**.

## Daily loop (what to expect)

1. Add Easy/Medium problems in **My List** (Hard is tracked but not scheduled).
2. **Today** assigns a small budget (Easy=1, Medium=2).
3. Solve on LeetCode (or open from Today) and/or rate manually with **Again / Hard / Medium / Easy**.
4. FSRS sets the next due date; rated items show on Today, and due items can return on later days.

## Checks before you push

```sh
npm run check
```

That runs format, lint, typecheck, unit tests, extension tests, Playwright e2e, frontend/extension builds, and Rust tests.

Pull requests are also gated by GitHub Actions (`.github/workflows/ci.yml`) so breaking changes fail CI before merge.

## Build notes

`npm run tauri -- build` produces an **unsigned internal macOS build**. Code signing, notarization, and cloud sync are out of scope for this MVP.
