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
- Keeps your access token in the app's own configuration directory, in a file
  only you can read. It never leaves your machine, and you only have to paste
  it once.
- Has themes, which set the colours and the sound together. *Automatic* follows
  the season — Halloween in October, Christmas in December — and the system's
  light or dark appearance for the rest of the year. *Match System* always
  follows the system's appearance and never brings a season, and *Halloween*
  and *Christmas* can be picked all year round.
- Lets you turn the themed sounds off, so the chime stays the same whatever
  the theme is doing.

It only notifies when reviews *newly* become available. A queue that stays
pending does not nag you, and the notification comes back after you clear it.
Tanuki also announces reviews that are already waiting when it starts, so a
restart never hides a queue from you.

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
  credential.rs         where the token is kept
  settings.rs           the user's preferences
  theme.rs              the themes the user can pick

src-tauri/              the Tauri application
  poller.rs             the background polling loop
  tray.rs               the menu bar icon and its menu
  audio.rs              plays the chime
  state.rs              settings and shared state

ui/                     the settings window (no build step)
```

The API token is kept in the app's own configuration directory, in a file only
its owner can read, rather than in the system's credential store. On macOS a
keychain item is tied to the application that created it, so as soon as the
app is rebuilt or upgraded macOS asks the user for their password before the
token can be read again — and if that prompt is dismissed rather than always
allowed, the token has to be typed in again from scratch. A read-only API token
does not need that ceremony, and the file survives every upgrade untouched.

The app polls `GET /v2/assignments?immediately_available_for_review` for the
authoritative count of waiting reviews, and `GET /v2/summary` to learn when the
next batch unlocks so it can sleep until then instead of guessing. Both
requests carry an `ETag`, so repeated polls are answered with a cheap
`304 Not Modified`.

The detector is the piece that keeps the app from being annoying. It
remembers whether reviews were pending and only reports a change on the
transition from "nothing waiting" to "something waiting". It starts fresh on
every launch, which is why reviews that are already waiting are announced when
Tanuki starts.

A theme is a set of colours and a chime. The colours live in the interface as
CSS variables, and the chime is rendered from the notes the theme names, so
adding another season means adding a theme rather than picking over the app.

Each look has its own voice. The plain one is a two-note bell: a bright upper
note and a lower one. Halloween is a sour, slow fall that creeps up on you
over most of a second. Christmas is a run of sleigh bells going up, each one
a separate strike, and then one that rings out.

There are two things to a theme. The user picks a *theme* — Automatic, Match
System, Halloween or Christmas — and that is turned into the *look* that is
actually in use. The automatic theme follows the season, so it is Halloween in
October and Christmas in December; the match system theme never is. The look
decides the colours and the chime, and the app works it out on its own, which
is why a theme that was never picked still changes on the first of the month.

The user can also turn the themed sounds off. The colours still follow the
theme, but the standard bell is played whatever the theme is doing.

## Development

```sh
cargo tauri dev
```

This builds the Rust side and serves `ui/` directly.

### Testing against a local server

Two environment variables help when you are working on the app itself:

- `TANUKI_API_URL` points the client at a different API root, for example
  `http://127.0.0.1:8731/v2`.
- `TANUKI_TEST_TOKEN` puts a token into the store at startup, so you do not
  have to click through the settings window.

Neither is used in normal operation.

## Licence

MIT.
