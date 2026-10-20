// The settings window talks to the Rust side through Tauri's invoke API.
// `withGlobalTauri` is on, so the API is available on `window.__TAURI__`
// without a bundler or any build step.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const elements = {
  status: document.getElementById("status"),
  statusDot: document.getElementById("status-dot"),
  statusText: document.getElementById("status-text"),
  token: document.getElementById("token"),
  saveToken: document.getElementById("save-token"),
  tokenLink: document.getElementById("token-link"),
  tokenState: document.getElementById("token-state"),
  sound: document.getElementById("sound"),
  notification: document.getElementById("notification"),
  launchAtLogin: document.getElementById("launch-at-login"),
  notifyLessons: document.getElementById("notify-lessons"),
  test: document.getElementById("test"),
  check: document.getElementById("check"),
};

/** Shows a short message under the token field. */
function message(text, isError = false) {
  elements.tokenState.textContent = text;
  elements.tokenState.classList.toggle("error", isError);
}

/** Reads the current value of every toggle. */
function readSettings() {
  return {
    sound: elements.sound.checked,
    notification: elements.notification.checked,
    launch_at_login: elements.launchAtLogin.checked,
    notify_lessons: elements.notifyLessons.checked,
  };
}

/** Fills the toggles with the saved settings. */
function writeSettings(settings) {
  elements.sound.checked = settings.sound;
  elements.notification.checked = settings.notification;
  elements.launchAtLogin.checked = settings.launch_at_login;
  elements.notifyLessons.checked = settings.notify_lessons;
}

/** Loads the saved settings and the token state. */
async function load() {
  const current = await invoke("get_settings");
  writeSettings(current.settings);

  if (current.configured) {
    elements.token.placeholder = "••••••••  (a token is saved)";
    message("A token is saved. Enter a new one to replace it.");
  } else {
    message("No token saved yet.");
  }
}
/** Saves the settings whenever a toggle changes. */
function watchToggles() {
  for (const element of [
    elements.sound,
    elements.notification,
    elements.launchAtLogin,
    elements.notifyLessons,
  ]) {
    element.addEventListener("change", async () => {
      try {
        await invoke("save_settings", { settings: readSettings() });
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
  } catch (error) {
    message(String(error), true);
  } finally {
    elements.saveToken.disabled = false;
  }
}

/** Updates the status line. */
function setStatus(status) {
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

elements.check.addEventListener("click", () => {
  invoke("check_now");
});

listen("status", (event) => setStatus(event.payload));

// The settings are loaded before the listeners are attached, so that filling
// the toggles in does not look like the user changing them.
load()
  .then(watchToggles)
  .catch((error) => message(String(error), true));
