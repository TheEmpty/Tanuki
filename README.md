# Tanuki

An unofficial third-party WaniKani review notifier.

Tanuki lives in your menu bar (macOS) or notification area (Windows). It
watches your WaniKani review queue and tells you — with a chime and a system
notification — the moment new reviews are waiting for you.

Tanuki is not affiliated with, endorsed by, or made by WaniKani. It is a
third-party app that uses only your own personal access token, and it reads
nothing but your review queue.

## Written by an AI

Tanuki is written and maintained entirely by an AI coding agent. There is no
hand-written code in it: the Rust, the interface, the tests, the CI and the
releases are all produced by an AI working from the notes in
[AGENTS.md](AGENTS.md). A human decides what should be built and says when it
is finished, but the building is done by the agent — including the bug fixes
and the documentation.

If you find a problem, that is the sort of thing an agent can fix, and it
will be fixed the same way.

## What it does

- Watches the WaniKani API for reviews that are ready to be done.
- Plays a short bell chime and posts a system notification when that happens.
- Notices when you level up, and celebrates it with a fanfare of its own.
  You can turn that off if you would rather not be told.
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
- Keeps running when you press Command-Q. The window goes away and Tanuki
  goes back to the menu bar, because a notifier that stops watching the
  review queue because of a stray keypress is no use at all. The first time
  you do it, Tanuki says so, and points at its icon in the menu bar. You can
  turn the setting on to make Command-Q quit as usual, or quit it from the
  tray menu, or with Command-Option-Q.

It only notifies when reviews *newly* become available. A queue that stays
pending does not nag you, and the notification comes back after you clear it.
Tanuki also announces reviews that are already waiting when it starts, so a
restart never hides a queue from you.

## Requirements

- A [WaniKani personal access token][token]. You can create one under
  *Settings → API Tokens* on the website.
- macOS 10.15 or later, or Windows 10 or later.

[token]: https://www.wanikani.com/settings/personal_access_tokens

## Installing

Download the installer for your platform from the
[releases page](https://github.com/TheEmpty/Tanuki/releases), and put Tanuki in
your applications folder.

The app is not signed, so it will not open without a little persuasion:

- **macOS.** Right-click Tanuki.app and choose *Open*, then *Open* again. Or
  run `xattr -dr com.apple.quarantine /Applications/Tanuki.app` to take the
  quarantine mark off it.
- **Windows.** "Windows protected your PC" means exactly that it does not know
  the publisher. Choose *More info* and then *Run anyway*.

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

### Releasing

The version lives in two places, and they have to agree: `version` in
`Cargo.toml`, and `version` in `src-tauri/tauri.conf.json`.

To publish a release:

1. Write the release notes at the top of `CHANGELOG.md`.
2. Bump the version in both places.
3. Commit, and tag the commit: `git tag v0.2.0`.
4. Push the tag: `git push origin main --tags`.

The [release workflow](.github/workflows/release.yml) then checks the code on
Linux, builds the macOS and Windows bundles on their own runners, and creates
the release with the notes from the changelog. Nothing is signed, so both
installers need the persuasion described above.

### Icons

The icons are generated from the SVG sources in `assets/`:

```sh
cargo tauri icon assets/icon.svg                 # the app icon
cargo tauri icon assets/tray-template.svg -o src-tauri/tray-icons --png 44
mv src-tauri/tray-icons/44x44.png src-tauri/tray-icons/macos-template.png
```

The menu bar icon is a *template* image: macOS recolours it so it stays
legible in light and dark menu bars. Windows gets the full colour mascot
instead.

The template image is 44 by 44, which macOS draws at 22 points. The
viewBox in `assets/tray-template.svg` is cropped tightly around the
drawing so the head fills the height of the icon, the way the other
menu bar icons do.

## How it works

```
crates/notifier-core/   pure Rust, no user interface
  api.rs                the WaniKani API client
  detect.rs             decides when reviews count as "new"
  level.rs              notices when the user has levelled up
  chime.rs              synthesises the bell and the fanfare
  token.rs               where the token is kept
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
its owner can read. The file survives every upgrade untouched, so the token
only has to be pasted in once.

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
CSS variables, the mascot in the settings window changes with them, and the
chime is rendered from the notes the theme names, so adding another season
means adding a theme rather than picking over the app. The seasonal looks
bring their own mascot too: a pumpkin for Halloween, and the tanuki in a
Santa hat for Christmas.

Each look has its own voice. The plain one is a two-note bell: a bright upper
note and a lower one. Halloween is a sour, slow fall that creeps up on you
over most of a second. Christmas is a run of sleigh bells going up, each one
a separate strike, and then one that rings out.

Levelling up has a fanfare of its own, in the manner of an old role playing
game: a run up to a big landing that rings out, played on a pulse wave the
way a console would have. It is the same whatever the theme is doing, since
no theme has a better way of saying congratulations.

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
