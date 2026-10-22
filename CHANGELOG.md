# Changelog

## 0.2.0

![The Tanuki settings window](https://raw.githubusercontent.com/TheEmpty/Tanuki/main/docs/settings.png)

The first release of Tanuki.

Tanuki is an unofficial third-party WaniKani review notifier for macOS and
Windows. It lives in the menu bar, watches your review queue, and tells you
with a chime and a notification when reviews are ready.

> **Tanuki is written and maintained entirely by an AI coding agent.** There
> is no hand-written code in it. The Rust, the interface, the tests, the CI
> and these releases were all produced by an AI working from the notes in
> `AGENTS.md`; a human decides what should be built and says when it is
> finished. Bug fixes and documentation are done the same way.

- Watches the WaniKani API for reviews that are ready to be done, and plays a
  short bell and posts a system notification when they are.
- Announces new lessons as well, if you want it to.
- Congratulates you when you level up, with a fanfare of its own. You can turn
  that off if you would rather not be told.
- Themes, which set the colours and the sound together. *Automatic* follows the
  season, so it is Halloween in October and Christmas in December, and the
  system appearance for the rest of the year. *Match System* always follows the
  system and never brings a season, and *Halloween* and *Christmas* can be
  picked all year round.
- A seasonal mascot: the tanuki is a pumpkin for Halloween, and wears a Santa
  hat for Christmas.
- The API token is kept in the app's own configuration directory, in a file
  only you can read.
- Command-Q puts the window away instead of quitting, so a stray keypress
  cannot stop the app watching your review queue. Tanuki says so the first
  time, and points at its icon in the menu bar. You can turn that off in the
  settings, or quit from the tray menu.

### Installing

The app is not signed, so macOS will not open it without a little persuasion:
right-click it in your Downloads folder and choose *Open*, then *Open* again.
Windows will say "Windows protected your PC"; choose *More info* and then *Run
anyway*.

You will need a
[personal access token](https://www.wanikani.com/settings/personal_access_tokens);
paste it into the settings window and Tanuki will remember it.
