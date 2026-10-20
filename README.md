# Tanuki

An unofficial third-party WaniKani review notifier.

Tanuki lives in your menu bar (macOS) or notification area (Windows). It
watches your WaniKani review queue and tells you — with a chime and a system
notification — the moment new reviews are waiting for you.

Tanuki is not affiliated with, endorsed by, or made by WaniKani. It is a
third-party app that uses only your own personal access token, and it reads
nothing but your review queue.

## What it does

- Watches the WaniKani API for reviews that are ready to be done.
- Plays a short bell chime and posts a system notification when that happens.
- Shows the number of waiting reviews in the tray menu.
- Can start automatically when you log in.
- Keeps your access token in your system's credential store (Keychain on
  macOS, Credential Manager on Windows) rather than in a file.

It only notifies when reviews *newly* become available. A queue that stays
pending does not nag you, and the notification comes back after you clear it.

## Requirements

- A [WaniKani personal access token][token]. You can create one under
  *Settings → API Tokens* on the website.
- macOS 10.15 or later, or Windows 10 or later.

[token]: https://www.wanikani.com/settings/personal_access_tokens

## Building

The project is a Cargo workspace. You need a Rust toolchain; the interface is
plain HTML, CSS and JavaScript, so there is no Node.js requirement.

```sh
cargo build --release          # the binary
cargo test -p notifier-core    # the tests
```

To produce installers, you need the Tauri CLI:

```sh
cargo install tauri-cli --version "^2.12"
cargo tauri build
```

That writes:

- macOS: `target/release/bundle/macos/Tanuki.app` and a `.dmg`
- Windows: `target/release/bundle/msi/*.msi` and `bundle/nsis/*.exe`

Windows installers have to be built on Windows; the WiX and NSIS toolchains
are not available on other platforms.

### Icons

The icons are generated from the SVG sources in `assets/`:

```sh
cargo tauri icon assets/icon.svg                 # the app icon
cargo tauri icon assets/tray-template.svg -o src-tauri/tray-icons --png 44
```

The menu bar icon is a *template* image: macOS recolours it so it stays
legible in light and dark menu bars. Windows gets the full colour mascot
instead.

## How it works

```
crates/notifier-core/   pure Rust, no user interface
  api.rs                the WaniKani API client
  detect.rs             decides when reviews count as "new"
  chime.rs              synthesises the bell
  credential.rs         the token store
  settings.rs           the user's preferences

src-tauri/              the Tauri application
  poller.rs             the background polling loop
  tray.rs               the menu bar icon and its menu
  audio.rs              plays the chime
  state.rs              settings and shared state

ui/                     the settings window (no build step)
```

The app polls `GET /v2/assignments?immediately_available_for_review` for the
authoritative count of waiting reviews, and `GET /v2/summary` to learn when the
next batch unlocks so it can sleep until then instead of guessing. Both
requests carry an `ETag`, so repeated polls are answered with a cheap
`304 Not Modified`.

The detector is the piece that keeps the app from being annoying. It
remembers whether reviews were pending and only reports a change on the
transition from "nothing waiting" to "something waiting". Its state is saved
to disk, so restarting Tanuki does not re-announce reviews you already know
about.

## Development

```sh
cargo tauri dev
```

This builds the Rust side and serves `ui/` directly.

## Licence

MIT.
