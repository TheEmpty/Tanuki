// The settings window talks to the Rust side through Tauri's invoke API.
// `withGlobalTauri` is on, so the API is available on `window.__TAURI__`
// without a bundler or any build step.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// The mascot each look wears.
const MASCOTS = {
  default: "mascot.svg",
  halloween: "mascot-halloween.svg",
  christmas: "mascot-christmas.svg",
};

const elements = {
  mascot: document.getElementById("mascot"),
  status: document.getElementById("status"),
  statusDot: document.getElementById("status-dot"),
  statusText: document.getElementById("status-text"),
  token: document.getElementById("token"),
  saveToken: document.getElementById("save-token"),
  tokenLink: document.getElementById("token-link"),
  tokenState: document.getElementById("token-state"),
  sound: document.getElementById("sound"),
  themedSounds: document.getElementById("themed-sounds"),
  notification: document.getElementById("notification"),
  launchAtLogin: document.getElementById("launch-at-login"),
  notifyLessons: document.getElementById("notify-lessons"),
  quitWithCommandQ: document.getElementById("quit-with-command-q"),
  theme: document.getElementById("theme"),
  test: document.getElementById("test"),
};

/** Shows a short message under the token field. */
function message(text, isError = false) {
  elements.tokenState.textContent = text;
  elements.tokenState.classList.toggle("error", isError);
}

/**
 * Puts a look on the window: its colours and the mascot it wears.
 */
function applyLook(look) {
  document.documentElement.setAttribute("data-look", look);

  const mascot = MASCOTS[look] || MASCOTS.default;
  if (elements.mascot.getAttribute("src") !== mascot) {
    elements.mascot.setAttribute("src", mascot);
  }
}

/** Reads the current value of every setting. */
function readSettings() {
  return {
    sound: elements.sound.checked,
    notification: elements.notification.checked,
    launch_at_login: elements.launchAtLogin.checked,
    notify_lessons: elements.notifyLessons.checked,
    quit_with_command_q: elements.quitWithCommandQ.checked,
    theme: elements.theme.value,
    themed_sounds: elements.themedSounds.checked,
  };
}

/** Fills the settings with the saved ones. */
function writeSettings(settings) {
  elements.sound.checked = settings.sound;
  elements.themedSounds.checked = settings.themed_sounds;
  elements.notification.checked = settings.notification;
  elements.launchAtLogin.checked = settings.launch_at_login;
  elements.notifyLessons.checked = settings.notify_lessons;
  elements.quitWithCommandQ.checked = settings.quit_with_command_q;
  elements.theme.value = settings.theme;

  // A theme from a newer version of the app is not in the list, so fall
  // back to the first one rather than leaving the picker empty.
  if (!elements.theme.value) {
    elements.theme.value = elements.theme.options[0].value;
  }
}

/** Fills the theme picker with the themes the app offers. */
async function fillThemes() {
  for (const theme of await invoke("get_themes")) {
    const option = document.createElement("option");
    option.value = theme.id;
    option.textContent = theme.name;
    elements.theme.append(option);
  }
}

/** Loads the saved settings and the token state. */
async function load() {
  await fillThemes();

  const current = await invoke("get_settings");
  writeSettings(current.settings);
  applyLook(current.look);

  if (current.configured) {
    elements.token.placeholder = "••••••••  (a token is saved)";
    message("A token is saved. Enter a new one to replace it.");
    // Show what is waiting straight away instead of waiting for the next poll.
    setStatus(await invoke("get_status"));
  } else {
    message("No token saved yet.");
  }
}

/** Saves the settings whenever one of them changes. */
function watchToggles() {
  for (const element of [
    elements.sound,
    elements.themedSounds,
    elements.notification,
    elements.launchAtLogin,
    elements.notifyLessons,
    elements.quitWithCommandQ,
    elements.theme,
  ]) {
    element.addEventListener("change", async () => {
      try {
        // The app answers with the look that is now in use, so the window
        // changes colour as soon as the setting is picked.
        const look = await invoke("save_settings", { settings: readSettings() });
        applyLook(look);
      } catch (error) {
        message(String(error), true);
      }
    });
  }
}

/** Saves the API token. */
async function saveToken() {
  const token = elements.token.value.trim();
  if (!token) {
    message("Enter a personal access token first.", true);
    return;
  }

  elements.saveToken.disabled = true;
  try {
    await invoke("save_token", { token });
    elements.token.value = "";
    elements.token.placeholder = "••••••••  (a token is saved)";
    message("Token saved.");
    // The status bubble only makes sense once there is a token to poll with.
    elements.status.hidden = false;
  } catch (error) {
    message(String(error), true);
  } finally {
    elements.saveToken.disabled = false;
  }
}

/** Updates the status line. */
function setStatus(status) {
  // Without a token there is nothing to report, so the bubble stays hidden.
  if (!status.configured) {
    elements.status.hidden = true;
    return;
  }

  elements.status.hidden = false;

  if (status.error) {
    elements.statusDot.className = "status-dot error";
    elements.statusText.textContent = status.error;
    return;
  }

  if (status.reviews > 0) {
    elements.statusDot.className = "status-dot ready";
    elements.statusText.textContent =
      status.reviews === 1
        ? "1 review is waiting."
        : `${status.reviews} reviews are waiting.`;
  } else {
    elements.statusDot.className = "status-dot";
    elements.statusText.textContent = "No reviews waiting.";
  }

  if (status.next_review_at) {
    const when = new Date(status.next_review_at);
    elements.statusText.textContent += ` Next reviews at ${when.toLocaleTimeString(
      [],
      { hour: "2-digit", minute: "2-digit" },
    )}.`;
  }
}

elements.saveToken.addEventListener("click", saveToken);

elements.token.addEventListener("keydown", (event) => {
  if (event.key === "Enter") {
    saveToken();
  }
});

elements.tokenLink.addEventListener("click", (event) => {
  // Open in the real browser rather than inside the window.
  event.preventDefault();
  invoke("open_token_page");
});

elements.test.addEventListener("click", () => {
  invoke("test_notification");
});

listen("status", (event) => {
  setStatus(event.payload);
  // The look comes with every status, so the window keeps up with the
  // season without having to be reopened.
  applyLook(event.payload.look);
});

// The settings are loaded before the listeners are attached, so that filling
// the toggles in does not look like the user changing them.
load()
  .then(watchToggles)
  .catch((error) => message(String(error), true));
