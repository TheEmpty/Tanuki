# AGENTS.md

Notes for anyone — human or otherwise — working on Tanuki.

The README covers what the app does and how it is put together. This file
covers the things that are not obvious from the code, mostly the traps that
cost somebody an afternoon.

## The shape of it

```
crates/notifier-core/   plain Rust, no user interface, fully tested
  api.rs                the WaniKani client
  chime.rs              the bell, the spooky fall, the jingle, the fanfare
  token.rs              where the API token is kept
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
  tray-icons/           the menu bar icon, and the mascot Windows shows

assets/                 the SVG sources the icons are generated from
ui/                     the settings window, no build step
docs/                   the newest screenshot; the older ones live in their tags
dev/mockapi.py          a stand-in for the WaniKani API
dev/release-notes.py    writes the notes for a release
```

The split is deliberate: `notifier-core` has no idea a window or an audio
device exists, so the interesting logic can be tested without either. Keep it
that way — if something in `notifier-core` starts needing Tauri, it belongs
somewhere else.

## Commands

```sh
cargo fmt --all -- --check                           # reports, does not rewrite
cargo clippy --workspace --all-targets -- --deny warnings
cargo test --workspace                              # 69 tests, a few seconds
cargo build -p tanuki --release                     # the binary, about three minutes
```

Those are the commands the workflows run, flags and all. Everything after
`--` goes to the tool rather than to cargo, and `--deny warnings` is what
makes clippy exit non-zero instead of printing and carrying on — so a run of
these four that is clean is a run that CI will agree with.

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

`TANUKI_TEST_TOKEN` puts a token in place at startup instead, which saves
typing it in.

The mock never sends an `ETag` and never answers `304 Not Modified`, so it
does not exercise the client's caching at all. A change to the caching will
pass here whatever it does to the real thing.

## Traps

### Windows opens a terminal for the app

A program built for Windows is given a console unless it says that it is a
windowed program, and a console means a terminal window beside the app.
`main.rs` says so with `windows_subsystem`, which is why there is only the
app. A debug build keeps the terminal, because that is where the log goes.

### There is no Windows machine here

Tanuki is written on a Mac, and nothing on this side of the desk can start it
on Windows. Somebody who has a Windows machine does run it, which is how the
Windows fixes were found, but the checking that happens here is the
compile-time trick below.

**The Windows build was broken for the whole of the first version.** The
Command-Q menu, the handling of it, and the warning that Tanuki is still
running are all macOS things, so the code for them sits behind
`#[cfg(target_os = "macos")]` — and `update_command_q_item` reached for
`menu::HIDE` outside that guard. Nothing on a Mac ever noticed, because the
code it was reaching for was there.

The way to catch that sort of thing without a Windows machine is to turn the
conditional compilation the other way round and compile:

```sh
sed -i '' 's/target_os = "macos"/target_os = "windows"/g' src-tauri/src/*.rs
cargo clippy --workspace --all-targets -- --deny warnings
```

That is not a real Windows build, but it catches the two mistakes that
matter: a name that only exists on one platform, and a function that nothing
calls on the other. Put it back afterwards.

**Tauri on Linux needs the GTK and WebKit development packages.** The
`Check` job in the workflows runs on macOS rather than Linux for that
reason. A Linux runner has none of them, so `cargo clippy` fails in
`glib-sys` before it reaches any of Tanuki's own code:

```sh
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev build-essential
```

Tanuki does not ship for Linux, so this is only worth doing if you are
changing something that has to keep working there.

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

Only macOS does this. Windows puts the notification up whether the app is in
front or not, so the test button leaves the window where it is there. The
window going away was reported by a Windows user as the window vanishing for
no reason.

The same applies to any real notification — when Tanuki is in the menu bar
it is not frontmost, so those do banner.

### The Notification Center panel being open swallows banners

If the Notification Center or the Control Centre panel is open, banners do
not appear and the notification only goes into the panel. It is easy to
leave a panel open by accident, and then it looks like notifications are
broken when they are not. `killall NotificationCenter` closes it.

### A dropped audio sink stops the sound

`rodio`'s `MixerDeviceSink` owns the stream that writes to the device. The
`Player` alone is not enough: if the sink is dropped, playback ends
immediately and silently. `audio.rs` holds the sink in `_sink` for the life
of the thread for this reason. The underscore is deliberate — it is there
to be kept, not read.

The device is opened the first time something is played rather than when the
app starts, so a machine with no working output device can still run Tanuki.

### The app is an accessory app, so it has no menu bar of its own

`LSUIElement` is set, so Tanuki has no Dock icon and no visible menu. The
application menu it builds exists purely for its keyboard shortcuts, and
`Command-Q` is the one that matters.

## Conventions

### Making a release

**Always ask before cutting a release.** A tag makes a release, and a
release is public and cannot be taken back quietly. Say what the release
would contain and wait to be told to go ahead, even when the fixes are
ready and the tests are green.

Every release page has the same shape, because somebody who has never seen
the app may land on the notes for any one of them:

1. A screenshot of that version, at the top.
2. A blurb about what the application is and does.
3. The notes for that version.

The blurb is the text between the title and the first version in
`CHANGELOG.md`, and the screenshot is the first line of each version's part
of it. `dev/release-notes.py` puts the three in that order, and the release
workflow runs it.

The screenshot is fetched from the release's **tag**, not from `main`:

    https://raw.githubusercontent.com/TheEmpty/Tanuki/v0.2.2/docs/screenshots/0.2.2.png

That is why the picture has to be committed before the tag is made, and why
only the newest one is kept on `main` — it is the one the README shows. A
release page keeps the picture it was published with even after the file is
removed from `main`, because the tag still has it.

The path has to be a full URL, not a repository-relative one like the
README's: GitHub renders the release body itself, and a relative path there
is a broken image.

To make a release:

1. Write the notes for the version at the top of `CHANGELOG.md`, with its
   screenshot on the first line of them.
2. Take the screenshot from that version's build and put it in
   `docs/screenshots/<version>.png`, scaled to 600 pixels high, and point
   the README at it. There is a section on taking it below.
3. Bump the version in `Cargo.toml` and in `src-tauri/tauri.conf.json`.
4. Commit, and tag the commit: `git tag v0.2.2`.
5. Push the tag: `git push origin main --tags`.
6. Remove the previous version's screenshot from `main`; its own tag still
   has it.

The [release workflow](.github/workflows/release.yml) then checks the code
on macOS, builds the macOS and Windows bundles on their own runners, and
creates the release. Nothing is signed, so both installers need the
persuasion described in the README.

It reads the notes from `CHANGELOG.md` and stops with an error if there is no
`## <version>` section for the tag, so the two are written together.

### Changing a release that is already out

Editing `CHANGELOG.md` changes nothing on a release page. The notes are the
release's body, and the workflow only writes them when it creates the
release, so they go up again by hand:

```sh
python3 dev/release-notes.py v0.2.2 > /tmp/notes.md
gh release edit v0.2.2 --notes-file /tmp/notes.md
```

`--notes-file` replaces the body and leaves the four installers attached, so
nothing has to be rebuilt. Then look at the page rather than assuming: a
wrong screenshot URL is a broken image on a page nobody opens twice.

```sh
gh release view v0.2.2                  # the notes are there, and four installers
gh release list                         # still published, still the latest
curl -s -o /tmp/shot.png -w '%{http_code}\n' \
  https://raw.githubusercontent.com/TheEmpty/Tanuki/v0.2.2/docs/screenshots/0.2.2.png
sips -g pixelHeight /tmp/shot.png       # 600
```

### Taking the screenshot for a release

It has to come from that version's build, not from the working tree: a
release page showing a version that was never released is worse than one
with no picture.

```sh
hdiutil attach Tanuki_0.2.2_universal.dmg
open /Volumes/Tanuki/Tanuki.app
```

Put a token in place and point the app at `dev/mockapi.py` (see *Testing
against a local API*), with a review count in the file that is worth looking
at — seven is what the releases so far show. The app may not poll for a
while, so ask it to check now from the tray menu, then open the settings
window from the same menu.

```sh
screencapture -o -W /tmp/tanuki.png    # click the window; -o leaves out the shadow
sips -Z 600 600 /tmp/tanuki.png
```

`hdiutil detach /Volumes/Tanuki` afterwards, and take the token back out of
the configuration directory.

### Adding a theme

A theme is a *look*, and the difference is deliberate. The user picks a
theme; the theme is turned into the look that is actually in use, because
the automatic theme follows the season. The look decides the colours and
the chime, and the app works it out on its own.

To add one:

1. `theme.rs` — add the variant to `Theme` and a row to `THEMES` with its
   identifier and the name the interface shows. `THEMES` is the one place a
   theme is written down: `id()`, `name()` and `from_id()` all read it, and
   so does the dropdown. Then add the arm to `resolve()`, which the compiler
   will insist on anyway.
2. `theme.rs` — if it needs a look of its own, add that variant to `Look`
   and a row to `LOOKS` in the same way.
3. `chime.rs` — give the look a voice in `voice()`. The fanfare is not part
   of this: levelling up sounds the same whatever the theme is doing.
4. `ui/style.css` — add a `[data-look="..."]` block with the same variables
   as the other looks. Every look has one, including the default, whose
   block is the one that follows the system's light or dark appearance.
5. `ui/app.js` — add the mascot to `MASCOTS` if it has its own.
6. `ui/` — add the mascot as an SVG if it needs one.

A variant left out of `THEMES` or `LOOKS` cannot be picked and has no
identifier; `every_theme_is_described_and_offered_in_this_order` and
`every_look_is_described` fail and say so, which is why those two tests
spell their lists out instead of counting them.

The variables each look sets are: `--bg`, `--card`, `--card-edge`,
`--card-edge-hover`, `--ink`, `--ink-dim`, `--accent`, `--accent-hover`,
`--on-accent`, `--danger` and `--heading`. The names describe the role, not
the colour, so a look can pick whatever it likes.

### The interface is a fixed set of files

`ui/index.html`, `ui/app.js` and `ui/style.css` are served as they are.
There is no bundler, so there is no import map either — the script is
plain `<script src="app.js">` and it uses `window.__TAURI__` directly.

### The configuration directory holds three files

On macOS it is `~/Library/Application Support/com.tanuki.notifier`, and
Windows has an equivalent under `AppData`:

- `settings.json` — the user's preferences, written by
  `AppState::save_settings`.
- `state.json` — what the app remembers between runs: the level the user was
  last seen at, and whether it has said that Command-Q does not quit. It is
  written by one method, `save_remembered`, so the two cannot disagree, and
  the locks are released before it is called.
- `token` — the API token, in a file of its own that only its owner can read.

Keep the token out of `settings.json`. It is not worth much to anyone else,
but users paste `settings.json` into issues, and a token in it would go with
them.

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
