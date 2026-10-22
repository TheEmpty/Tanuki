# AGENT.md

Notes for anyone — human or otherwise — working on Tanuki.

The README covers what the app does and how it is put together. This file
covers the things that are not obvious from the code, mostly the traps that
cost somebody an afternoon.

## The shape of it

```
crates/notifier-core/   plain Rust, no user interface, fully tested
  api.rs                the WaniKani client
  chime.rs              the bell, the spooky fall, the jingle, the fanfare
  credential.rs         where the API token is kept
  detect.rs             decides when reviews count as "new"
  level.rs              notices when the user has levelled up
  settings.rs           the user's preferences
  theme.rs              the themes the user can pick, and the looks they mean

src-tauri/              the Tauri application
  audio.rs              plays the chime on a thread of its own
  lib.rs                the commands, the wiring, and the settings window
  menu.rs               the application menu, which is invisible
  poller.rs             the background polling loop
  state.rs              shared state, and what is remembered between runs
  tray.rs               the menu bar icon

ui/                     the settings window, no build step
dev/mockapi.py          a stand-in for the WaniKani API
```

The split is deliberate: `notifier-core` has no idea a window or an audio
device exists, so the interesting logic can be tested without either. Keep it
that way — if something in `notifier-core` starts needing Tauri, it belongs
somewhere else.

## Commands

```sh
cargo test --workspace              # 68 tests, a few seconds
cargo clippy --workspace --all-targets
cargo build -p tanuki --release     # the binary, about three minutes
```

`cargo tauri build --bundles app` makes the `.app`. The Tauri CLI is not on
`PATH` in this environment; it lives at
`/private/var/folders/zc/hpj4q7tx29s2tx_42sgc06pm0000gn/T/opencode/cargo-tauri`.
There is no `node`, `npm` or `bun` here, which is why the interface is plain
HTML, CSS and JavaScript with no build step.

## Testing against a local API

`dev/mockapi.py` serves the three endpoints the app uses. The review count
comes from a file, so it can be changed while the app is running, and the
level from another:

```sh
python3 dev/mockapi.py /tmp/tanuki-count 8731 /tmp/tanuki-requests.log &
echo 7 > /tmp/tanuki-count
echo 30 > /tmp/tanuki-level

mkdir -p ~/Library/Application\ Support/com.tanuki.notifier
echo a-token > ~/Library/Application\ Support/com.tanuki.notifier/token
chmod 600 ~/Library/Application\ Support/com.tanuki.notifier/token

TANUKI_API_URL=http://127.0.0.1:8731/v2 ./target/release/tanuki
```

`TANUKI_TEST_TOKEN` puts a token in place at startup instead, which is
handy but writes to the keychain and will ask for permission.

## Traps

### The interface is cached, in two places

**Cargo will not notice an edit to a file in `ui/`.** `tauri-build` asks
Cargo to watch the *directory* the interface lives in, and Cargo only notices
a change to the directory itself, not to the files inside it. An edited page
was therefore quietly ignored and the old one built into the app.
`src-tauri/build.rs` now watches the files themselves — keep that.

**`cargo build` and `cargo tauri build` keep separate copies of the
interface.** They compile with different feature sets, so they have
different build script fingerprints and different output directories. A stale
copy in one is not refreshed by a build of the other. If the app shows
something that is not in `ui/index.html`, rebuild the thing you are about to
run, and check the other one.

**WebKit does not cache the interface, and the app makes sure of it.** Tauri
sends no cache headers and no validators for its asset protocol, so there is
nothing for a browser to cache — a page loaded from a previous build is
always fetched afresh. This was checked by building the app twice with a
marker in the interface: the second build showed the marker, so the earlier
"stale interface" was the build script above and never the cache.

The app still sends `Cache-Control: no-store` on every response, so that this
cannot change under it. That is the reason the settings window is built in
`lib.rs` rather than by Tauri: `on_web_resource_request` is the only hook
that can add a header to an asset response, and it is only available on a
window the app builds itself. The configuration therefore says `create:
false` for the window, and `create_main_window` builds it from that same
configuration. Keep the two in step — the window's size, title and position
still live in `tauri.conf.json`.

### macOS will not banner a notification from the app in front

If the app is frontmost, a notification goes straight into Notification
Center and no banner appears. This is why the test button puts the window
away first: otherwise the test can only be seen by opening Notification
Center, which is not much of a test.

The same applies to any real notification — when Tanuki is in the menu bar
it is not frontmost, so those do banner.

### The Notification Center panel being open swallows banners

If the Notification Center or the Control Centre panel is open, banners do
not appear and the notification only goes into the panel. It is easy to
leave a panel open by accident, and then it looks like notifications are
broken when they are not. `killall NotificationCenter` closes it.

### An unsigned app cannot keep a keychain item across a rebuild

The app is not signed, and macOS ties a keychain item to the code that
created it. Every rebuild changes the code, so the item can no longer be
read without a prompt — and if the prompt is dismissed rather than always
allowed, the token has to be typed in again. That is why the token lives in
a file in the app's own configuration directory, with permissions only its
owner can read, and why the keychain is no longer used at all.

### A dropped audio sink stops the sound

`rodio`'s `DeviceSink` owns the stream that writes to the device. The
`Player` alone is not enough: if the sink is dropped, playback ends
immediately and silently. `audio.rs` holds the sink in `_sink` for the life
of the thread for this reason. The underscore is deliberate — it is there
to be kept, not read.

### A keychain read at startup blocks the app

Reading the credential store can put a password prompt on screen, and the
app blocks in `setup()` until it is answered. The token is cached in memory
after the first read so this happens at most once.

### The app is an accessory app, so it has no menu bar of its own

`LSUIElement` is set, so Tanuki has no Dock icon and no visible menu. The
application menu it builds exists purely for its keyboard shortcuts, and
`Command-Q` is the one that matters.

## Conventions

### Adding a theme

A theme is a *look*, and the difference is deliberate. The user picks a
theme; the theme is turned into the look that is actually in use, because
the automatic theme follows the season. The look decides the colours and
the chime, and the app works it out on its own.

To add one:

1. `theme.rs` — add the variant, its identifier, its name, and what it
   resolves to. `Look` is the palette, `Theme` is what the user picks.
2. `chime.rs` — add a voice and give it to the look in `voice()`.
3. `ui/style.css` — add a `[data-look="..."]` block with the same variables
   as the other looks.
4. `ui/app.js` — add the mascot to `MASCOTS` if it has its own.
5. `ui/` — add the mascot as an SVG if it needs one.

The variables each look sets are: `--bg`, `--card`, `--card-edge`,
`--card-edge-hover`, `--ink`, `--ink-dim`, `--accent`, `--accent-hover`,
`--on-accent`, `--danger` and `--heading`. The names describe the role, not
the colour, so a look can pick whatever it likes.

### The interface is a fixed set of files

`ui/index.html`, `ui/app.js` and `ui/style.css` are served as they are.
There is no bundler, so there is no import map either — the script is
plain `<script src="app.js">` and it uses `window.__TAURI__` directly.

### Keep the state file for things only the app knows

`state.json` in the configuration directory holds what the app remembers
between runs: the level the user was last seen at, and whether it has said
that Command-Q does not quit. It is written by one method so the two
cannot disagree, and the locks are released before it is called.

## Known outstanding issues

**The window is tall.** It is 460 by 840, which is most of a laptop screen.
Every setting has a row with a description, and they add up. It scrolls if
you make it shorter, but the test button and the footer are meant to be
visible without scrolling.

**There is no way to see the app's own log.** It goes to standard error,
which is invisible for an accessory app. `Console.app` shows the system
log but not standard error from a bundled app.

## Looking at things

The settings window is the quickest way to see a change. To look at an SVG
without launching anything:

```sh
qlmanage -t -s 512 -o /tmp ui/mascot-halloween.svg
```

To see the menu bar icon against the real thing, run the app and take a
screenshot — `screencapture -x file.png` — then crop it.

The app's menu bar icon is a template image, so macOS recolours it for
light and dark menu bars. The tray menu is reached by clicking the icon.
