(function () {
  "use strict";

  var HOST_ID = "localcoder-chat-widget";
  var STYLE_ID = "localcoder-chat-style";
  var STATUS_URL = "/api/localcoder/status";
  var CHAT_URL = "/api/localcoder/chat";
  var FALLBACK_MESSAGE =
    "Localcoder is not available from this page yet. Any prompt you send stays in this transcript, and you can try again after the local service is running.";

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

  function statusLabelFromPayload(payload) {
    if (!payload || typeof payload !== "object") {
      return "Localcoder ready";
    }

    if (typeof payload.status === "string" && payload.status.trim()) {
      return payload.status.trim();
    }

    if (typeof payload.message === "string" && payload.message.trim()) {
      return payload.message.trim();
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
    var requestOptions = options || {};

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

        if (!response.ok) {
          var statusError = new Error("Request failed with status " + response.status);
          statusError.status = response.status;
          statusError.missingRoute = response.status === 404 || response.status === 405;
          throw statusError;
        }

        if (contentType.indexOf("application/json") !== -1) {
          return response.json();
        }

        return response.text().then(function (text) {
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

  function buildWidget() {
    var existing = document.getElementById(HOST_ID);
    var host;
    var root;
    var style;
    var panel;
    var header;
    var titleWrap;
    var title;
    var statusRow;
    var statusDot;
    var statusText;
    var toggleButton;
    var transcript;
    var form;
    var input;
    var sendButton;
    var state = {
      available: false,
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
    style.id = STYLE_ID;
    style.textContent = [
      ":host {",
      "  color-scheme: light dark;",
      "  --lc-bg: #f9faf7;",
      "  --lc-panel: #ffffff;",
      "  --lc-ink: #16201b;",
      "  --lc-muted: #5d6962;",
      "  --lc-border: #cfd7d1;",
      "  --lc-user: #e5f0ff;",
      "  --lc-assistant: #edf7ee;",
      "  --lc-system: #fff5d6;",
      "  --lc-accent: #16685a;",
      "  --lc-accent-strong: #0e4d43;",
      "  --lc-offline: #a84832;",
      "  --lc-wait: #9a6a13;",
      "  --lc-shadow: 0 18px 45px rgba(18, 24, 20, 0.22);",
      "  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;",
      "  position: fixed;",
      "  right: 18px;",
      "  bottom: 18px;",
      "  z-index: 2147483000;",
      "}",
      ":host, :host * { box-sizing: border-box; }",
      ".lc-panel {",
      "  width: min(360px, calc(100vw - 32px));",
      "  max-height: min(600px, calc(100vh - 36px));",
      "  display: grid;",
      "  grid-template-rows: auto minmax(180px, 1fr) auto;",
      "  overflow: hidden;",
      "  color: var(--lc-ink);",
      "  background: var(--lc-panel);",
      "  border: 1px solid var(--lc-border);",
      "  border-radius: 8px;",
      "  box-shadow: var(--lc-shadow);",
      "}",
      ".lc-panel.is-collapsed {",
      "  grid-template-rows: auto;",
      "  width: min(310px, calc(100vw - 32px));",
      "}",
      ".lc-panel.is-collapsed .lc-transcript,",
      ".lc-panel.is-collapsed .lc-form { display: none; }",
      ".lc-header {",
      "  min-height: 56px;",
      "  display: flex;",
      "  align-items: center;",
      "  justify-content: space-between;",
      "  gap: 12px;",
      "  padding: 10px 12px;",
      "  border-bottom: 1px solid var(--lc-border);",
      "  background: var(--lc-bg);",
      "}",
      ".lc-title-wrap { min-width: 0; }",
      ".lc-title {",
      "  margin: 0;",
      "  font-size: 14px;",
      "  line-height: 18px;",
      "  font-weight: 700;",
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
      "  box-shadow: 0 0 0 3px rgba(154, 106, 19, 0.16);",
      "}",
      ".lc-status-text {",
      "  overflow: hidden;",
      "  text-overflow: ellipsis;",
      "  white-space: nowrap;",
      "}",
      ".lc-panel.is-ready .lc-dot {",
      "  background: var(--lc-accent);",
      "  box-shadow: 0 0 0 3px rgba(22, 104, 90, 0.16);",
      "}",
      ".lc-panel.is-offline .lc-dot {",
      "  background: var(--lc-offline);",
      "  box-shadow: 0 0 0 3px rgba(168, 72, 50, 0.16);",
      "}",
      ".lc-toggle {",
      "  width: 34px;",
      "  height: 34px;",
      "  flex: 0 0 34px;",
      "  display: inline-flex;",
      "  align-items: center;",
      "  justify-content: center;",
      "  color: var(--lc-ink);",
      "  background: transparent;",
      "  border: 1px solid transparent;",
      "  border-radius: 6px;",
      "  cursor: pointer;",
      "  font-size: 18px;",
      "  line-height: 1;",
      "}",
      ".lc-toggle:hover, .lc-toggle:focus-visible {",
      "  border-color: var(--lc-border);",
      "  background: #eef3ef;",
      "  outline: none;",
      "}",
      ".lc-transcript {",
      "  min-height: 180px;",
      "  max-height: 410px;",
      "  overflow-y: auto;",
      "  display: flex;",
      "  flex-direction: column;",
      "  gap: 8px;",
      "  padding: 12px;",
      "  background: #fbfcfa;",
      "}",
      ".lc-message {",
      "  width: fit-content;",
      "  max-width: 92%;",
      "  padding: 8px 10px;",
      "  border: 1px solid var(--lc-border);",
      "  border-radius: 8px;",
      "  color: var(--lc-ink);",
      "  background: var(--lc-assistant);",
      "  font-size: 13px;",
      "  line-height: 1.4;",
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
      "  color: #4f3e08;",
      "  background: var(--lc-system);",
      "}",
      ".lc-form {",
      "  display: grid;",
      "  grid-template-columns: minmax(0, 1fr) auto;",
      "  gap: 8px;",
      "  padding: 10px;",
      "  border-top: 1px solid var(--lc-border);",
      "  background: var(--lc-bg);",
      "}",
      ".lc-input {",
      "  width: 100%;",
      "  min-height: 42px;",
      "  max-height: 150px;",
      "  resize: vertical;",
      "  padding: 9px 10px;",
      "  color: var(--lc-ink);",
      "  background: #ffffff;",
      "  border: 1px solid var(--lc-border);",
      "  border-radius: 6px;",
      "  font: inherit;",
      "  font-size: 13px;",
      "  line-height: 1.35;",
      "}",
      ".lc-input:focus {",
      "  border-color: var(--lc-accent);",
      "  outline: 2px solid rgba(22, 104, 90, 0.2);",
      "  outline-offset: 1px;",
      "}",
      ".lc-send {",
      "  min-width: 68px;",
      "  height: 42px;",
      "  align-self: end;",
      "  padding: 0 14px;",
      "  color: #ffffff;",
      "  background: var(--lc-accent);",
      "  border: 1px solid var(--lc-accent-strong);",
      "  border-radius: 6px;",
      "  font: inherit;",
      "  font-size: 13px;",
      "  font-weight: 700;",
      "  cursor: pointer;",
      "}",
      ".lc-send:hover, .lc-send:focus-visible {",
      "  background: var(--lc-accent-strong);",
      "  outline: none;",
      "}",
      ".lc-send:disabled {",
      "  cursor: not-allowed;",
      "  color: rgba(255, 255, 255, 0.78);",
      "  background: #78948d;",
      "  border-color: #78948d;",
      "}",
      "@media (prefers-color-scheme: dark) {",
      "  :host {",
      "    --lc-bg: #161b18;",
      "    --lc-panel: #202721;",
      "    --lc-ink: #f1f5ef;",
      "    --lc-muted: #b8c4ba;",
      "    --lc-border: #3b473f;",
      "    --lc-user: #18324b;",
      "    --lc-assistant: #1e3a31;",
      "    --lc-system: #493d18;",
      "    --lc-accent: #2ea88f;",
      "    --lc-accent-strong: #55c6af;",
      "    --lc-offline: #e07a62;",
      "    --lc-wait: #d4a54b;",
      "    --lc-shadow: 0 18px 45px rgba(0, 0, 0, 0.45);",
      "  }",
      "  .lc-header, .lc-form { background: var(--lc-bg); }",
      "  .lc-transcript { background: #171c19; }",
      "  .lc-input { background: #111512; }",
      "  .lc-message.is-system { color: #ffe7a0; }",
      "  .lc-toggle:hover, .lc-toggle:focus-visible { background: #27302a; }",
      "  .lc-send { color: #07110e; }",
      "  .lc-send:disabled { color: rgba(7, 17, 14, 0.7); }",
      "}",
      "@media (max-width: 520px) {",
      "  :host {",
      "    right: 8px;",
      "    bottom: 8px;",
      "    left: 8px;",
      "  }",
      "  .lc-panel, .lc-panel.is-collapsed {",
      "    width: 100%;",
      "    max-height: calc(100vh - 16px);",
      "  }",
      "  .lc-form { grid-template-columns: 1fr; }",
      "  .lc-send { width: 100%; }",
      "}",
      "@media (prefers-reduced-motion: no-preference) {",
      "  .lc-panel { transition: width 140ms ease, max-height 140ms ease; }",
      "  .lc-send, .lc-toggle { transition: background-color 120ms ease, border-color 120ms ease; }",
      "}"
    ].join("\n");

    panel = createElement("div", "lc-panel is-waiting");
    header = createElement("div", "lc-header");
    titleWrap = createElement("div", "lc-title-wrap");
    title = createElement("h2", "lc-title", "Localcoder");
    statusRow = createElement("div", "lc-status");
    statusDot = createElement("span", "lc-dot");
    statusText = createElement("span", "lc-status-text", "Checking local service");
    toggleButton = createElement("button", "lc-toggle", "-");
    transcript = createElement("div", "lc-transcript");
    form = createElement("form", "lc-form");
    input = createElement("textarea", "lc-input");
    sendButton = createElement("button", "lc-send", "Send");

    title.id = "localcoder-chat-title";
    statusDot.setAttribute("aria-hidden", "true");
    statusText.setAttribute("role", "status");
    statusText.setAttribute("aria-live", "polite");
    toggleButton.type = "button";
    toggleButton.setAttribute("aria-label", "Collapse Localcoder chat");
    toggleButton.title = "Collapse";
    transcript.setAttribute("aria-live", "polite");
    transcript.setAttribute("aria-label", "Localcoder transcript");
    transcript.setAttribute("role", "log");
    input.placeholder = "Ask localcoder";
    input.rows = 2;
    input.autocomplete = "off";
    input.spellcheck = true;
    input.setAttribute("aria-label", "Prompt for localcoder");
    sendButton.type = "submit";
    sendButton.disabled = true;

    statusRow.append(statusDot, statusText);
    titleWrap.append(title, statusRow);
    header.append(titleWrap, toggleButton);
    form.append(input, sendButton);
    panel.append(header, transcript, form);
    root.append(style, panel);

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
      if (!isCollapsed) {
        window.setTimeout(function () {
          input.focus();
          scrollTranscript();
        }, 0);
      }
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
          body: JSON.stringify({ prompt: prompt })
        },
        60000
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
          state.available = false;
          setStatus(
            "offline",
            isMissingRoute(error) ? "Localcoder unavailable" : "Localcoder request failed"
          );
          showMissingNotice();
        })
        .finally(function () {
          setSending(false);
          if (!state.collapsed) {
            input.focus();
          }
        });
    }

    toggleButton.addEventListener("click", function () {
      setCollapsed(!state.collapsed);
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
      },
      close: function () {
        setCollapsed(true);
      },
      refreshStatus: refreshStatus
    };
  }

  onReady(buildWidget);
})();
