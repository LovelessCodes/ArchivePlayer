# ArchivePlayer

A local-first desktop media library for public-domain films and shows from
[Archive.org](https://archive.org). Browse and search the Internet Archive, build
your own shelves, and stream directly from their servers — while your catalog and
watch history stay on your computer.

This is a ground-up **Rust + Tauri** rewrite of
[ArchivePlayer by Jeppe Kjeldal](https://github.com/JeppeKjeldal/ArchivePlayer),
which was originally an ASP.NET Core MVC application. Prebuilt installers for
Windows, macOS, and Linux are available on the
[Releases](https://github.com/LovelessCodes/ArchivePlayer/releases) page.

> **Attribution:** This project is derived from
> [JeppeKjeldal/ArchivePlayer](https://github.com/JeppeKjeldal/ArchivePlayer).
> The original author is credited in the app's Settings → About screen and in the
> [LICENSE](LICENSE) file. The concept, feature set, and product design are based
> on that project.

## Features

- **Live Archive.org search** with video filtering and pagination.
- **Add by URL or identifier** for items you already know.
- **Custom categories** for organizing your library, with TMDB-powered suggestions.
- **Continue watching** with per-title and per-episode progress.
- **Episode selector** with season filtering and automatic next-episode playback.
- **Playback controls**: speed, picture-in-picture, fullscreen, 10-second skip,
  mute, keyboard shortcuts, and double-tap/double-click seeking.
- **Optional TMDB enrichment**: posters, genres, episode titles, descriptions, and
  category suggestions.
- **Filename-based episode detection** when TMDB is not configured.
- **Local profiles** with Argon2 password hashing and an in-process session.
- **Local SQLite database** — no account server, telemetry, or cloud dependency.
- **Clear watch history** or remove individual titles at any time.
- A dark, responsive UI built with React 19, Tailwind CSS v4, and shadcn/ui
  components on [Base UI](https://base-ui.com/).

## Tech stack

| Layer | Technology |
| --- | --- |
| Desktop shell | Tauri 2 |
| Backend | Rust 2024 edition |
| Frontend | React 19, TypeScript, Vite |
| Styling | Tailwind CSS v4, shadcn/ui (Base UI primitives) |
| Data fetching | TanStack Query |
| Database | SQLite via `rusqlite` (bundled) |
| Passwords | Argon2id |
| HTTP | `reqwest` with rustls |
| Metadata | Archive.org, optional TMDB v3 API |

## Getting started

### Prerequisites

- [Rust](https://rustup.rs/) 1.90 or newer
- Node.js 22 or newer and [pnpm](https://pnpm.io/)
- Tauri system dependencies for your platform. On Debian/Ubuntu:

  ```bash
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
    librsvg2-dev libsoup-3.0-dev build-essential curl wget file
  ```

  See the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/)
  for macOS and Windows.

### Run the desktop app

```bash
pnpm install
pnpm tauri dev
```

The first Rust build takes a while. Tauri starts Vite automatically.

### Run the UI in a browser

`pnpm dev` opens the React app at `http://localhost:1420` with a small
in-memory/localStorage mock backend. This is useful for frontend work without
compiling Rust:

```bash
pnpm dev
```

The browser demo seeds a sample library after you create a local profile. The
desktop app always uses the real Rust backend.

### Checks and tests

```bash
pnpm check                 # TypeScript type check
pnpm build                 # production frontend build
cargo test --workspace     # Rust unit tests
cargo clippy --workspace --all-targets
```

### Build installers

```bash
pnpm tauri build
```

Bundles are written to `target/release/bundle/`. Use
`pnpm tauri build --no-bundle` to produce just the binary.

### Releases

Releases are built by GitHub Actions when a version tag is pushed:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The tag must match the version in `package.json` and
`src-tauri/tauri.conf.json`; the workflow fails early if they differ. It builds
and attaches:

- **Windows x64** — NSIS `.exe` and `.msi`
- **macOS Intel** (`x86_64`) — `.dmg`
- **macOS Apple Silicon** (`aarch64`) — `.dmg`
- **Linux x64** — `.deb` and `.AppImage`
- **Linux ARM64** (`aarch64`) — `.deb`

The installers are currently **unsigned**. macOS may require right-click → **Open**
the first time, and Windows SmartScreen may warn. Linux AppImage users may need
to run `chmod +x` before launching.

## How it works

```text
┌────────────────────────────────────────────────────────────┐
│ Tauri window                                               │
│  React 19 + Tailwind + shadcn/ui                           │
│    ↕ typed invoke() calls                                  │
│  src-tauri: Tauri commands, session state, window setup    │
│    ↕                                                       │
│  crates/archiveplayer-core                                 │
│    auth · settings · SQLite · Archive.org · TMDB · playback │
└────────────────────────────────────────────────────────────┘
```

- `src/` — React app, routes, feature pages, UI components, browser mock.
- `src-tauri/` — Tauri entry point, commands, capabilities, and packaging config.
- `crates/archiveplayer-core/` — framework-independent Rust domain logic:
  - `db.rs` + `migrations/` — users, library, categories, progress.
  - `archive.rs` — Archive.org advanced search and metadata.
  - `tmdb.rs` — optional TMDB matching, posters, and episodes.
  - `episodes.rs` — filename-based episode detection and matching.
  - `service.rs` — orchestration used by the Tauri commands.

### Local data

The app stores its database and settings in the Tauri application data directory:

- **Linux:** `~/.local/share/io.github.jeppekjeldal.archiveplayer/`
- **macOS:** `~/Library/Application Support/io.github.jeppekjeldal.archiveplayer/`
- **Windows:** `%APPDATA%\io.github.jeppekjeldal.archiveplayer\`

Files:

- `archiveplayer.db` — profiles, library, categories, watch history.
- `settings.json` — last email and optional TMDB key (written with `0600`
  permissions on Unix).

Set `ARCHIVEPLAYER_DATA_DIR` to override the location (useful for portable setups
or testing).

### Profiles

Profiles are local to the machine. The session is held in memory, so closing the
app signs you out; the library is preserved. Passwords are hashed with Argon2id;
no plaintext password is stored.

### TMDB (optional)

ArchivePlayer works without any API key. To enable richer metadata:

1. Create a free account at [themoviedb.org](https://www.themoviedb.org/signup).
2. Request a v3 API key under **Settings → API**.
3. Paste it into **Settings → TMDB metadata** in ArchivePlayer, then use
   **Test connection**.

The key is stored only in the local `settings.json` file and is sent only to
`api.themoviedb.org`. Without a key, the app uses Archive.org thumbnails and
filename-based episode parsing.

## Playback and privacy

- Video is streamed from Archive.org; nothing is downloaded to your library
  folder. Availability and playback depend on Archive.org and the codecs
  supported by the system webview (WebKitGTK on Linux, WKWebView on macOS,
  WebView2 on Windows).
- Network access is limited to Archive.org, TMDB (only with a key), and the
  external links you explicitly open.
- There is no analytics, telemetry, or remote account system.
- ArchivePlayer is not affiliated with the Internet Archive or TMDB.

## Known limitations

- Playback of every Archive.org format is not guaranteed; `.mp4`, `.webm`, and
  formats supported by the platform webview are the most reliable. The app
  prefers `.mp4` files when an item has several video formats.
- Episode matching is heuristic when TMDB is not configured or cannot find a
  match.
- The app manages a local single-machine library; it does not sync profiles or
  watch history between devices.
- Archive.org items can be removed or restricted by their uploaders at any time.

## License and attribution

Released under the [MIT License with Attribution Requirement](LICENSE).

This project is a derivative rewrite of
[ArchivePlayer](https://github.com/JeppeKjeldal/ArchivePlayer) by
**Jeppe Kjeldal**. The original project's attribution request is preserved here,
in the application's About screen, and in the repository history:

- Credit the original author: **Jeppe Kjeldal**.
- Link to the original repository:
  <https://github.com/JeppeKjeldal/ArchivePlayer>.
- Note that this work is derived from that project.

See [LICENSE](LICENSE) for the full text.
