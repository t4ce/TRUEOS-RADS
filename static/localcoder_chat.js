(function () {
  "use strict";

  var HOST_ID = "localcoder-chat-widget";
  var STATUS_URL = "/api/localcoder/status";
  var CHAT_URL = "/api/localcoder/chat";
  var FALLBACK_MESSAGE =
    "Localcoder is not available from this page yet. Prompts stay in this transcript until the local service is ready.";

  if (window.__localcoderChatLoaded) {
    return;
  }
  window.__localcoderChatLoaded = true;

  function onReady(callback) {
    if (document.readyState === "complete") {
      window.setTimeout(callback, 0);
      return;
    }
    window.addEventListener("load", callback, { once: true });
  }

  function createElement(tagName, className, text) {
    var element = document.createElement(tagName);
    if (className) {
      element.className = className;
    }
    if (typeof text === "string") {
      element.textContent = text;
    }
    return element;
  }

  function normalizeText(value) {
    return String(value == null ? "" : value).replace(/\r\n/g, "\n");
  }

  function hasVisibleText(value) {
    return normalizeText(value).trim().length > 0;
  }

  function isMissingRoute(error) {
    return Boolean(
      error &&
        (error.missingRoute ||
          error.status === 404 ||
          error.status === 405 ||
          error.name === "TypeError")
    );
  }

  function titleCase(value) {
    var text = normalizeText(value).trim();
    if (!text) {
      return "";
    }
    return text.charAt(0).toUpperCase() + text.slice(1);
  }

  function statusLabelFromPayload(payload) {
    var settings;
    var provider;
    var model;

    if (!payload || typeof payload !== "object") {
      return "Localcoder ready";
    }

    if (typeof payload.status === "string" && payload.status.trim()) {
      return payload.status.trim();
    }

    if (typeof payload.message === "string" && payload.message.trim()) {
      return payload.message.trim();
    }

    settings = payload.settings || {};
    provider = titleCase(settings.provider);
    model = normalizeText(settings.model).trim();
    if (provider && model) {
      return provider + " / " + model;
    }

    if (typeof payload.model === "string" && payload.model.trim()) {
      return "Localcoder ready: " + payload.model.trim();
    }

    return "Localcoder ready";
  }

  function isAvailableStatus(payload) {
    if (!payload || typeof payload !== "object") {
      return true;
    }

    if (payload.available === false || payload.ready === false || payload.online === false) {
      return false;
    }

    if (typeof payload.status === "string") {
      var status = payload.status.toLowerCase();
      if (
        status.indexOf("offline") !== -1 ||
        status.indexOf("unavailable") !== -1 ||
        status.indexOf("error") !== -1
      ) {
        return false;
      }
    }

    return true;
  }

  function extractChatText(payload) {
    var choices;
    var firstChoice;

    if (typeof payload === "string") {
      return payload;
    }

    if (!payload || typeof payload !== "object") {
      return "";
    }

    if (typeof payload.reply === "string") {
      return payload.reply;
    }
    if (typeof payload.response === "string") {
      return payload.response;
    }
    if (typeof payload.message === "string") {
      return payload.message;
    }
    if (typeof payload.text === "string") {
      return payload.text;
    }
    if (typeof payload.content === "string") {
      return payload.content;
    }

    choices = payload.choices;
    if (Array.isArray(choices) && choices.length > 0) {
      firstChoice = choices[0];
      if (firstChoice && typeof firstChoice.text === "string") {
        return firstChoice.text;
      }
      if (
        firstChoice &&
        firstChoice.message &&
        typeof firstChoice.message.content === "string"
      ) {
        return firstChoice.message.content;
      }
    }

    return "";
  }

  function requestJson(url, options, timeoutMs) {
    var controller = typeof AbortController === "function" ? new AbortController() : null;
    var timeoutId = null;
    var requestOptions = Object.assign({}, options || {});

    if (typeof fetch !== "function") {
      return Promise.reject(new Error("Fetch is not available in this browser"));
    }

    if (controller) {
      requestOptions.signal = controller.signal;
      timeoutId = window.setTimeout(function () {
        controller.abort();
      }, timeoutMs);
    }

    return fetch(url, requestOptions)
      .then(function (response) {
        var contentType = response.headers.get("content-type") || "";

        if (contentType.indexOf("application/json") !== -1) {
          return response.json().then(function (payload) {
            if (!response.ok) {
              throw requestError(response, payload);
            }
            return payload;
          });
        }

        return response.text().then(function (text) {
          var payload;

          if (!response.ok) {
            try {
              payload = text ? JSON.parse(text) : text;
            } catch (error) {
              payload = text;
            }
            throw requestError(response, payload);
          }

          if (!text) {
            return {};
          }

          try {
            return JSON.parse(text);
          } catch (error) {
            return text;
          }
        });
      })
      .finally(function () {
        if (timeoutId !== null) {
          window.clearTimeout(timeoutId);
        }
      });
  }

  function requestError(response, payload) {
    var error = new Error("Request failed with status " + response.status);
    error.status = response.status;
    error.missingRoute = response.status === 404 || response.status === 405;
    error.payload = payload;
    return error;
  }

  function errorMessage(error) {
    var payload = error && error.payload;
    var text = "";
    var stderrIndex;

    if (payload && typeof payload === "object") {
      text = normalizeText(payload.detail || payload.message || payload.error || "");
    } else if (typeof payload === "string") {
      text = normalizeText(payload);
    } else if (error && error.name === "AbortError") {
      text = "Localcoder request timed out.";
    }

    stderrIndex = text.indexOf("stderr=");
    if (stderrIndex !== -1) {
      text = text.slice(stderrIndex + "stderr=".length);
    }
    text = text.replace(/^❌\s*/gm, "").trim();

    if (!hasVisibleText(text)) {
      return isMissingRoute(error) ? "Localcoder unavailable" : "Localcoder request failed";
    }

    if (text.indexOf("missing OpenAI API key") !== -1) {
      return "OpenAI API key missing. Add OPENAI_API_KEY to .env.local and restart RADS.";
    }

    return text;
  }

  function buildWidget() {
    var existing = document.getElementById(HOST_ID);
    var host;
    var root;
    var style;
    var panel;
    var dragHandle;
    var header;
    var titleWrap;
    var title;
    var statusRow;
    var statusDot;
    var statusText;
    var toggleButton;
    var tabBar;
    var chatTabButton;
    var toolsTabButton;
    var modelTabButton;
    var content;
    var chatPanel;
    var toolsPanel;
    var modelPanel;
    var transcript;
    var toolList;
    var modelDetails;
    var form;
    var input;
    var sendButton;
    var resizeHandle;
    var state = {
      available: false,
      activeTab: "chat",
      collapsed: false,
      missingNoticeShown: false,
      sending: false
    };

    if (existing) {
      return;
    }

    host = document.createElement("section");
    host.id = HOST_ID;
    host.setAttribute("aria-label", "Localcoder chat");
    document.body.appendChild(host);

    root = host.attachShadow ? host.attachShadow({ mode: "open" }) : host;

    style = createElement("style");
    style.textContent = [
      ":host {",
      "  color-scheme: dark;",
      "  --lc-bg: #111315;",
      "  --lc-panel: #191d20;",
      "  --lc-panel-2: #22272b;",
      "  --lc-panel-3: #2a3035;",
      "  --lc-line: #343c42;",
      "  --lc-line-strong: #4b565f;",
      "  --lc-text: #eef2f3;",
      "  --lc-muted: #9faab0;",
      "  --lc-muted-2: #c2cacf;",
      "  --lc-accent: #31b77d;",
      "  --lc-accent-soft: #163325;",
      "  --lc-wait: #d6a833;",
      "  --lc-offline: #ff6b6b;",
      "  --lc-user: #18324b;",
      "  --lc-assistant: #1d3128;",
      "  --lc-system: #3a321a;",
      "  --lc-shadow: 0 22px 55px rgba(0, 0, 0, 0.46);",
      "  position: fixed;",
      "  right: 18px;",
      "  bottom: 18px;",
      "  z-index: 2147483000;",
      "  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;",
      "}",
      ":host, :host * { box-sizing: border-box; }",
      "button, input, textarea { font: inherit; }",
      ".lc-panel {",
      "  position: relative;",
      "  width: min(780px, calc(100vw - 32px));",
      "  height: min(390px, calc(100vh - 36px));",
      "  min-width: min(360px, calc(100vw - 32px));",
      "  min-height: 320px;",
      "  max-width: calc(100vw - 32px);",
      "  max-height: min(640px, calc(100vh - 36px));",
      "  display: grid;",
      "  grid-template-rows: auto auto auto minmax(190px, 1fr) auto;",
      "  overflow: hidden;",
      "  color: var(--lc-text);",
      "  background: var(--lc-panel);",
      "  border: 1px solid var(--lc-line);",
      "  border-radius: 8px;",
      "  box-shadow: var(--lc-shadow);",
      "}",
      ".lc-panel.is-collapsed {",
      "  grid-template-rows: auto auto;",
      "  width: min(310px, calc(100vw - 32px)) !important;",
      "  height: auto !important;",
      "  min-height: 0;",
      "}",
      ".lc-panel.is-collapsed .lc-tabs,",
      ".lc-panel.is-collapsed .lc-content,",
      ".lc-panel.is-collapsed .lc-form { display: none; }",
      ".lc-panel.is-collapsed .lc-resize { display: none; }",
      ".lc-drag {",
      "  height: 18px;",
      "  display: grid;",
      "  place-items: center;",
      "  cursor: grab;",
      "  background: #141719;",
      "  border-bottom: 1px solid var(--lc-line);",
      "  touch-action: none;",
      "}",
      ".lc-drag::before {",
      "  content: '';",
      "  width: 42px;",
      "  height: 4px;",
      "  border-radius: 999px;",
      "  background: var(--lc-line-strong);",
      "}",
      ".lc-drag:active { cursor: grabbing; }",
      ".lc-header {",
      "  min-height: 54px;",
      "  display: flex;",
      "  align-items: center;",
      "  justify-content: space-between;",
      "  gap: 12px;",
      "  padding: 10px 12px;",
      "  border-bottom: 1px solid var(--lc-line);",
      "  background: #141719;",
      "}",
      ".lc-title-wrap { min-width: 0; }",
      ".lc-title {",
      "  margin: 0;",
      "  font-size: 14px;",
      "  line-height: 18px;",
      "  font-weight: 900;",
      "  letter-spacing: 0;",
      "}",
      ".lc-status {",
      "  display: flex;",
      "  align-items: center;",
      "  gap: 6px;",
      "  min-width: 0;",
      "  margin-top: 2px;",
      "  color: var(--lc-muted);",
      "  font-size: 12px;",
      "  line-height: 16px;",
      "}",
      ".lc-dot {",
      "  width: 8px;",
      "  height: 8px;",
      "  flex: 0 0 8px;",
      "  border-radius: 50%;",
      "  background: var(--lc-wait);",
      "  box-shadow: 0 0 0 3px rgba(214, 168, 51, 0.14);",
      "}",
      ".lc-status-text {",
      "  overflow: hidden;",
      "  text-overflow: ellipsis;",
      "  white-space: nowrap;",
      "}",
      ".lc-panel.is-ready .lc-dot {",
      "  background: var(--lc-accent);",
      "  box-shadow: 0 0 0 3px rgba(49, 183, 125, 0.15);",
      "}",
      ".lc-panel.is-offline .lc-dot {",
      "  background: var(--lc-offline);",
      "  box-shadow: 0 0 0 3px rgba(255, 107, 107, 0.14);",
      "}",
      ".lc-toggle {",
      "  width: 34px;",
      "  height: 34px;",
      "  flex: 0 0 34px;",
      "  display: inline-flex;",
      "  align-items: center;",
      "  justify-content: center;",
      "  color: var(--lc-text);",
      "  background: var(--lc-panel-2);",
      "  border: 1px solid var(--lc-line);",
      "  border-radius: 6px;",
      "  cursor: pointer;",
      "  font-size: 18px;",
      "  line-height: 1;",
      "}",
      ".lc-toggle:hover, .lc-toggle:focus-visible {",
      "  border-color: var(--lc-line-strong);",
      "  background: var(--lc-panel-3);",
      "  outline: none;",
      "}",
      ".lc-tabs {",
      "  display: grid;",
      "  grid-template-columns: repeat(3, minmax(0, 1fr));",
      "  gap: 4px;",
      "  padding: 8px;",
      "  border-bottom: 1px solid var(--lc-line);",
      "  background: #151819;",
      "}",
      ".lc-tab {",
      "  min-width: 0;",
      "  min-height: 30px;",
      "  padding: 0 8px;",
      "  color: var(--lc-muted-2);",
      "  background: var(--lc-panel-2);",
      "  border: 1px solid var(--lc-line);",
      "  border-radius: 6px;",
      "  cursor: pointer;",
      "  font-size: 12px;",
      "  font-weight: 800;",
      "}",
      ".lc-tab:hover, .lc-tab:focus-visible {",
      "  border-color: var(--lc-line-strong);",
      "  background: var(--lc-panel-3);",
      "  outline: none;",
      "}",
      ".lc-tab.is-active {",
      "  color: #dcf8e7;",
      "  background: var(--lc-accent-soft);",
      "  border-color: #438465;",
      "}",
      ".lc-content {",
      "  min-height: 0;",
      "  background: #171b1e;",
      "}",
      ".lc-tab-panel {",
      "  height: 100%;",
      "  min-height: 0;",
      "}",
      ".lc-tab-panel[hidden] { display: none; }",
      ".lc-transcript {",
      "  height: 100%;",
      "  min-height: 190px;",
      "  overflow-y: auto;",
      "  display: flex;",
      "  flex-direction: column;",
      "  gap: 8px;",
      "  padding: 12px;",
      "}",
      ".lc-message {",
      "  width: fit-content;",
      "  max-width: 92%;",
      "  padding: 8px 10px;",
      "  border: 1px solid var(--lc-line);",
      "  border-radius: 8px;",
      "  color: var(--lc-text);",
      "  background: var(--lc-assistant);",
      "  font-size: 13px;",
      "  line-height: 1.42;",
      "  white-space: pre-wrap;",
      "  overflow-wrap: anywhere;",
      "}",
      ".lc-message.is-user {",
      "  align-self: flex-end;",
      "  background: var(--lc-user);",
      "}",
      ".lc-message.is-system {",
      "  align-self: center;",
      "  width: 100%;",
      "  max-width: 100%;",
      "  color: #ffe7a0;",
      "  background: var(--lc-system);",
      "}",
      ".lc-tools, .lc-model {",
      "  display: grid;",
      "  gap: 8px;",
      "  padding: 12px;",
      "}",
      ".lc-tools {",
      "  grid-template-columns: repeat(2, minmax(0, 1fr));",
      "}",
      ".lc-tool {",
      "  min-height: 34px;",
      "  display: grid;",
      "  grid-template-columns: auto minmax(0, 1fr);",
      "  gap: 8px;",
      "  padding: 7px 9px;",
      "  color: var(--lc-muted-2);",
      "  background: var(--lc-panel-2);",
      "  border: 1px solid var(--lc-line);",
      "  border-radius: 6px;",
      "  font-size: 12px;",
      "}",
      ".lc-tool.is-unavailable { opacity: 0.58; }",
      ".lc-tool input { accent-color: var(--lc-accent); }",
      ".lc-tool-name {",
      "  display: grid;",
      "  gap: 2px;",
      "  min-width: 0;",
      "}",
      ".lc-tool-name b {",
      "  overflow: hidden;",
      "  text-overflow: ellipsis;",
      "  white-space: nowrap;",
      "  font-size: 12px;",
      "}",
      ".lc-tool-name small {",
      "  overflow: hidden;",
      "  text-overflow: ellipsis;",
      "  white-space: nowrap;",
      "  color: var(--lc-muted);",
      "  font-size: 11px;",
      "}",
      ".lc-kv {",
      "  display: grid;",
      "  grid-template-columns: 86px minmax(0, 1fr);",
      "  gap: 8px;",
      "  align-items: start;",
      "  padding: 8px 9px;",
      "  color: var(--lc-muted-2);",
      "  background: var(--lc-panel-2);",
      "  border: 1px solid var(--lc-line);",
      "  border-radius: 6px;",
      "  font-size: 12px;",
      "  line-height: 1.35;",
      "}",
      ".lc-kv b {",
      "  color: var(--lc-muted);",
      "  font-weight: 800;",
      "}",
      ".lc-kv span {",
      "  min-width: 0;",
      "  overflow-wrap: anywhere;",
      "}",
      ".lc-form {",
      "  display: grid;",
      "  grid-template-columns: minmax(0, 1fr) auto;",
      "  gap: 8px;",
      "  padding: 10px;",
      "  border-top: 1px solid var(--lc-line);",
      "  background: #141719;",
      "}",
      ".lc-form[hidden] { display: none; }",
      ".lc-input {",
      "  width: 100%;",
      "  min-height: 42px;",
      "  max-height: 150px;",
      "  resize: vertical;",
      "  padding: 9px 10px;",
      "  color: var(--lc-text);",
      "  background: #111315;",
      "  border: 1px solid var(--lc-line);",
      "  border-radius: 6px;",
      "  font: inherit;",
      "  font-size: 13px;",
      "  line-height: 1.35;",
      "}",
      ".lc-input::placeholder { color: #737f86; }",
      ".lc-input:focus {",
      "  border-color: #438465;",
      "  outline: 2px solid rgba(49, 183, 125, 0.18);",
      "  outline-offset: 1px;",
      "}",
      ".lc-send {",
      "  min-width: 68px;",
      "  height: 42px;",
      "  align-self: end;",
      "  padding: 0 14px;",
      "  color: #07110c;",
      "  background: var(--lc-accent);",
      "  border: 1px solid #60d39e;",
      "  border-radius: 6px;",
      "  font: inherit;",
      "  font-size: 13px;",
      "  font-weight: 900;",
      "  cursor: pointer;",
      "}",
      ".lc-send:hover, .lc-send:focus-visible {",
      "  background: #54c994;",
      "  outline: none;",
      "}",
      ".lc-send:disabled {",
      "  cursor: not-allowed;",
      "  color: rgba(7, 17, 12, 0.68);",
      "  background: #5b806c;",
      "  border-color: #5b806c;",
      "}",
      ".lc-resize {",
      "  position: absolute;",
      "  left: 0;",
      "  bottom: 0;",
      "  width: 22px;",
      "  height: 22px;",
      "  z-index: 2;",
      "  cursor: nesw-resize;",
      "  touch-action: none;",
      "}",
      ".lc-resize::before {",
      "  content: '';",
      "  position: absolute;",
      "  left: 5px;",
      "  bottom: 5px;",
      "  width: 10px;",
      "  height: 10px;",
      "  border-left: 2px solid var(--lc-line-strong);",
      "  border-bottom: 2px solid var(--lc-line-strong);",
      "  opacity: 0.8;",
      "}",
      ".lc-resize:hover::before, .lc-resize:focus-visible::before {",
      "  border-color: var(--lc-accent);",
      "}",
      "@media (max-width: 520px) {",
      "  :host {",
      "    right: 8px;",
      "    bottom: 8px;",
      "    left: 8px;",
      "  }",
      "  .lc-panel, .lc-panel.is-collapsed {",
      "    width: 100%;",
      "    height: auto;",
      "    min-height: 0;",
      "    max-height: calc(100vh - 16px);",
      "  }",
      "  .lc-resize { display: none; }",
      "  .lc-form { grid-template-columns: 1fr; }",
      "  .lc-send { width: 100%; }",
      "}",
      "@media (prefers-reduced-motion: no-preference) {",
      "  .lc-panel { transition: width 140ms ease, max-height 140ms ease; }",
      "  .lc-message { animation: lc-message-in 180ms ease both; }",
      "  .lc-send, .lc-toggle, .lc-tab { transition: background-color 120ms ease, border-color 120ms ease; }",
      "}",
      "@keyframes lc-message-in {",
      "  from { opacity: 0; transform: translateY(5px); }",
      "  to { opacity: 1; transform: translateY(0); }",
      "}"
    ].join("\n");

    panel = createElement("div", "lc-panel is-waiting");
    dragHandle = createElement("div", "lc-drag");
    header = createElement("div", "lc-header");
    titleWrap = createElement("div", "lc-title-wrap");
    title = createElement("h2", "lc-title", "Localcoder");
    statusRow = createElement("div", "lc-status");
    statusDot = createElement("span", "lc-dot");
    statusText = createElement("span", "lc-status-text", "Checking local service");
    toggleButton = createElement("button", "lc-toggle", "-");
    tabBar = createElement("div", "lc-tabs");
    chatTabButton = createElement("button", "lc-tab is-active", "Chat");
    toolsTabButton = createElement("button", "lc-tab", "Tools");
    modelTabButton = createElement("button", "lc-tab", "Model");
    content = createElement("div", "lc-content");
    chatPanel = createElement("section", "lc-tab-panel");
    toolsPanel = createElement("section", "lc-tab-panel");
    modelPanel = createElement("section", "lc-tab-panel");
    transcript = createElement("div", "lc-transcript");
    toolList = createElement("div", "lc-tools");
    modelDetails = createElement("div", "lc-model");
    form = createElement("form", "lc-form");
    input = createElement("textarea", "lc-input");
    sendButton = createElement("button", "lc-send", "Send");
    resizeHandle = createElement("div", "lc-resize");

    title.id = "localcoder-chat-title";
    statusDot.setAttribute("aria-hidden", "true");
    statusText.setAttribute("role", "status");
    statusText.setAttribute("aria-live", "polite");
    toggleButton.type = "button";
    toggleButton.setAttribute("aria-label", "Collapse Localcoder chat");
    toggleButton.title = "Collapse";
    chatTabButton.type = "button";
    toolsTabButton.type = "button";
    modelTabButton.type = "button";
    chatTabButton.dataset.tab = "chat";
    toolsTabButton.dataset.tab = "tools";
    modelTabButton.dataset.tab = "model";
    transcript.setAttribute("aria-live", "polite");
    transcript.setAttribute("aria-label", "Localcoder transcript");
    transcript.setAttribute("role", "log");
    toolsPanel.hidden = true;
    modelPanel.hidden = true;
    input.placeholder = "Ask localcoder";
    input.rows = 2;
    input.autocomplete = "off";
    input.spellcheck = true;
    input.setAttribute("aria-label", "Prompt for localcoder");
    resizeHandle.setAttribute("aria-hidden", "true");
    sendButton.type = "submit";
    sendButton.disabled = true;

    statusRow.append(statusDot, statusText);
    titleWrap.append(title, statusRow);
    header.append(titleWrap, toggleButton);
    tabBar.append(chatTabButton, toolsTabButton, modelTabButton);
    chatPanel.append(transcript);
    toolsPanel.append(toolList);
    modelPanel.append(modelDetails);
    content.append(chatPanel, toolsPanel, modelPanel);
    form.append(input, sendButton);
    panel.append(dragHandle, header, tabBar, content, form, resizeHandle);
    root.append(style, panel);

    renderTools(null);
    renderModelDetails(null);
    makeDraggable(host, dragHandle);
    makeResizable(host, panel, resizeHandle);

    function scrollTranscript() {
      transcript.scrollTop = transcript.scrollHeight;
    }

    function appendMessage(kind, text) {
      var message = createElement("div", "lc-message");

      if (kind === "user") {
        message.className += " is-user";
      } else if (kind === "system") {
        message.className += " is-system";
      }

      message.textContent = normalizeText(text);
      transcript.appendChild(message);
      scrollTranscript();
      return message;
    }

    function showMissingNotice() {
      if (state.missingNoticeShown) {
        return;
      }
      state.missingNoticeShown = true;
      appendMessage("system", FALLBACK_MESSAGE);
    }

    function setStatus(mode, label) {
      panel.classList.remove("is-ready", "is-offline", "is-waiting");
      panel.classList.add(mode === "ready" ? "is-ready" : mode === "offline" ? "is-offline" : "is-waiting");
      statusText.textContent = label;
    }

    function setSending(isSending) {
      state.sending = isSending;
      sendButton.textContent = isSending ? "..." : "Send";
      updateSendState();
    }

    function updateSendState() {
      sendButton.disabled = state.sending || !hasVisibleText(input.value);
    }

    function setCollapsed(isCollapsed) {
      state.collapsed = isCollapsed;
      panel.classList.toggle("is-collapsed", isCollapsed);
      toggleButton.textContent = isCollapsed ? "+" : "-";
      toggleButton.title = isCollapsed ? "Expand" : "Collapse";
      toggleButton.setAttribute(
        "aria-label",
        isCollapsed ? "Expand Localcoder chat" : "Collapse Localcoder chat"
      );
      if (!isCollapsed && state.activeTab === "chat") {
        window.setTimeout(function () {
          input.focus();
          scrollTranscript();
        }, 0);
      }
    }

    function setTab(tab) {
      state.activeTab = tab;
      chatPanel.hidden = tab !== "chat";
      toolsPanel.hidden = tab !== "tools";
      modelPanel.hidden = tab !== "model";
      form.hidden = tab !== "chat";
      [chatTabButton, toolsTabButton, modelTabButton].forEach(function (button) {
        button.classList.toggle("is-active", button.dataset.tab === tab);
      });

      if (tab === "chat") {
        window.setTimeout(function () {
          input.focus();
          scrollTranscript();
        }, 0);
      }
    }

    function renderTools(payload) {
      toolList.textContent = "";
      localcoderTools(payload).forEach(function (tool) {
        var label = createElement("label", "lc-tool");
        var checkbox = createElement("input");
        var name = createElement("span", "lc-tool-name");
        var title = createElement("b", "", tool.label);
        var detail = createElement("small", "", tool.detail);

        if (!tool.available) {
          label.className += " is-unavailable";
        }
        checkbox.type = "checkbox";
        checkbox.checked = tool.available;
        checkbox.disabled = true;
        name.append(title, detail);
        label.title = tool.fullDetail || tool.detail;
        label.append(checkbox, name);
        toolList.appendChild(label);
      });
    }

    function localcoderTools(payload) {
      var tools = payload && Array.isArray(payload.tools) ? payload.tools : null;

      if (!tools) {
        return [
          { label: "Files", available: false, detail: "Read, Edit, Write" },
          { label: "Search", available: false, detail: "Glob, Grep" },
          { label: "Shell", available: false, detail: "Bash" },
          { label: "Web", available: false, detail: "WebFetch, WebSearch" },
          { label: "Git", available: false, detail: "via Bash git" },
          { label: "LSP", available: false, detail: "Lsp" },
          { label: "Plan", available: false, detail: "Plan tools" },
          { label: "Skills", available: false, detail: "Skill" }
        ];
      }

      return tools.map(function (tool) {
        var toolNames = Array.isArray(tool.tools) ? tool.tools.join(", ") : "";
        var detail = toolNames || normalizeText(tool.detail).trim() || normalizeText(tool.mode).trim();
        var mode = normalizeText(tool.mode).trim();
        if (mode === "via_shell") {
          detail = detail ? detail + " via Shell" : "via Shell";
        }
        return {
          label: normalizeText(tool.label || tool.id).trim() || "Tool",
          available: tool.available !== false,
          detail: detail,
          fullDetail: normalizeText(tool.detail).trim() || detail
        };
      });
    }

    function renderModelDetails(payload) {
      var settings = payload && payload.settings ? payload.settings : {};
      var provider = titleCase(settings.provider) || "Unknown";
      var model = normalizeText(settings.model).trim() || "Unknown";
      var endpoint = normalizeText(settings.base_url).trim() || "Unknown";
      var keyState = normalizeText(settings.api_key).trim() || "unknown";
      var keyLabel = {
        env: "OPENAI_API_KEY",
        missing: "Missing",
        not_required: "Not required",
        settings: "Settings file",
        unknown: "Unknown"
      }[keyState] || titleCase(keyState);

      modelDetails.textContent = "";
      appendModelRow("Provider", provider);
      appendModelRow("Model", model);
      appendModelRow("Endpoint", endpoint);
      appendModelRow("Key", keyLabel);
    }

    function appendModelRow(label, value) {
      var row = createElement("div", "lc-kv");
      var key = createElement("b", "", label);
      var text = createElement("span", "", value);
      row.append(key, text);
      modelDetails.appendChild(row);
    }

    function refreshStatus() {
      setStatus("waiting", "Checking local service");

      return requestJson(
        STATUS_URL,
        {
          method: "GET",
          headers: {
            Accept: "application/json"
          },
          cache: "no-store"
        },
        8000
      )
        .then(function (payload) {
          var label;

          renderModelDetails(payload);
          renderTools(payload);
          state.available = isAvailableStatus(payload);
          if (state.available) {
            setStatus("ready", statusLabelFromPayload(payload));
          } else {
            label = statusLabelFromPayload(payload);
            setStatus(
              "offline",
              label === "Localcoder ready" ? "Localcoder unavailable" : label
            );
            showMissingNotice();
          }
        })
        .catch(function (error) {
          state.available = false;
          renderModelDetails(null);
          renderTools(null);
          setStatus(
            "offline",
            isMissingRoute(error) ? "Localcoder unavailable" : "Localcoder status unknown"
          );
          showMissingNotice();
        });
    }

    function sendPrompt() {
      var prompt = normalizeText(input.value);

      if (!hasVisibleText(prompt) || state.sending) {
        return;
      }

      appendMessage("user", prompt);
      input.value = "";
      updateSendState();
      setSending(true);

      requestJson(
        CHAT_URL,
        {
          method: "POST",
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json"
          },
          body: JSON.stringify({ prompt: prompt, timeout_ms: 120000 })
        },
        130000
      )
        .then(function (payload) {
          var text = normalizeText(extractChatText(payload));
          state.available = true;
          setStatus("ready", "Localcoder ready");

          if (!hasVisibleText(text)) {
            appendMessage("system", "Localcoder replied, but the response did not include text.");
            return;
          }

          appendMessage("assistant", text);
        })
        .catch(function (error) {
          var message = errorMessage(error);

          state.available = false;
          setStatus("offline", message);
          appendMessage("system", message);
        })
        .finally(function () {
          setSending(false);
          if (!state.collapsed && state.activeTab === "chat") {
            input.focus();
          }
        });
    }

    toggleButton.addEventListener("click", function () {
      setCollapsed(!state.collapsed);
    });

    [chatTabButton, toolsTabButton, modelTabButton].forEach(function (button) {
      button.addEventListener("click", function () {
        setTab(button.dataset.tab);
      });
    });

    input.addEventListener("input", updateSendState);

    input.addEventListener("keydown", function (event) {
      if (event.key === "Enter" && !event.shiftKey) {
        event.preventDefault();
        sendPrompt();
      }
    });

    form.addEventListener("submit", function (event) {
      event.preventDefault();
      sendPrompt();
    });

    refreshStatus();

    window.LocalcoderChat = window.LocalcoderChat || {
      open: function () {
        setCollapsed(false);
        setTab("chat");
      },
      close: function () {
        setCollapsed(true);
      },
      refreshStatus: refreshStatus
    };
  }

  function makeDraggable(host, handle) {
    var dragging = false;
    var offsetX = 0;
    var offsetY = 0;

    handle.addEventListener("pointerdown", function (event) {
      var rect = host.getBoundingClientRect();
      dragging = true;
      offsetX = event.clientX - rect.left;
      offsetY = event.clientY - rect.top;
      handle.setPointerCapture(event.pointerId);
      event.preventDefault();
    });

    handle.addEventListener("pointermove", function (event) {
      var x;
      var y;

      if (!dragging) {
        return;
      }

      x = Math.max(8, Math.min(window.innerWidth - host.offsetWidth - 8, event.clientX - offsetX));
      y = Math.max(8, Math.min(window.innerHeight - host.offsetHeight - 8, event.clientY - offsetY));
      host.style.left = x + "px";
      host.style.top = y + "px";
      host.style.right = "auto";
      host.style.bottom = "auto";
    });

    handle.addEventListener("pointerup", function (event) {
      dragging = false;
      if (handle.hasPointerCapture(event.pointerId)) {
        handle.releasePointerCapture(event.pointerId);
      }
    });

    handle.addEventListener("pointercancel", function () {
      dragging = false;
    });
  }

  function makeResizable(host, panel, handle) {
    var resizing = false;
    var startX = 0;
    var startY = 0;
    var startWidth = 0;
    var startHeight = 0;
    var rightEdge = 0;
    var bottomEdge = 0;
    var usesLeftTop = false;
    var minWidth = 360;
    var minHeight = 320;
    var margin = 8;

    function clamp(value, min, max) {
      return Math.max(min, Math.min(max, value));
    }

    handle.addEventListener("pointerdown", function (event) {
      var rect = panel.getBoundingClientRect();

      resizing = true;
      startX = event.clientX;
      startY = event.clientY;
      startWidth = rect.width;
      startHeight = rect.height;
      rightEdge = rect.right;
      bottomEdge = rect.bottom;
      usesLeftTop = host.style.right === "auto" || host.style.bottom === "auto";
      handle.setPointerCapture(event.pointerId);
      event.preventDefault();
    });

    handle.addEventListener("pointermove", function (event) {
      var width;
      var height;
      var maxWidth;
      var maxHeight;

      if (!resizing) {
        return;
      }

      maxWidth = Math.max(minWidth, rightEdge - margin);
      maxHeight = Math.max(minHeight, bottomEdge - margin);
      width = clamp(startWidth + startX - event.clientX, minWidth, maxWidth);
      height = clamp(startHeight + startY - event.clientY, minHeight, maxHeight);

      panel.style.width = width + "px";
      panel.style.height = height + "px";

      if (usesLeftTop) {
        host.style.left = clamp(rightEdge - width, margin, window.innerWidth - width - margin) + "px";
        host.style.top = clamp(bottomEdge - height, margin, window.innerHeight - height - margin) + "px";
        host.style.right = "auto";
        host.style.bottom = "auto";
      }
    });

    handle.addEventListener("pointerup", function (event) {
      resizing = false;
      if (handle.hasPointerCapture(event.pointerId)) {
        handle.releasePointerCapture(event.pointerId);
      }
    });

    handle.addEventListener("pointercancel", function () {
      resizing = false;
    });
  }

  onReady(buildWidget);
})();
