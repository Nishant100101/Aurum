(() => {
  "use strict";

  const STORAGE_KEY = "expense_recorder_haptics";

  const PATTERNS = {
    selection: 30,
    light: 35,
    medium: 50,
    heavy: 70,
    success: [40, 60, 70],
    warning: [50, 60, 50],
    error: [60, 60, 60, 60, 90],
  };

  const TAP_MAP = [
    [".btn-danger, .btn-danger-solid", "warning"],
    [".btn", "medium"],
    [".chip, .date-chip", "selection"],
    [
      ".option-row, .expense-row, .icon-btn, .backdrop, .info-row-action, .dropzone",
      "light",
    ],
  ];

  const canVibrate = typeof navigator.vibrate === "function";
  const canSwitchHaptic =
    !canVibrate &&
    typeof CSS !== "undefined" &&
    typeof CSS.supports === "function" &&
    CSS.supports("-webkit-appearance", "switch");

  const telegramHaptic = (name) => {
    const tg = window.Telegram && window.Telegram.WebApp;
    if (!tg || !tg.initData || !tg.HapticFeedback) return false;
    try {
      if (name === "selection") {
        tg.HapticFeedback.selectionChanged();
      } else if (name === "success" || name === "warning" || name === "error") {
        tg.HapticFeedback.notificationOccurred(name);
      } else {
        tg.HapticFeedback.impactOccurred(
          name === "heavy" ? "heavy" : name === "medium" ? "medium" : "light",
        );
      }
      return true;
    } catch (_) {
      return false;
    }
  };

  let enabled = true;
  try {
    enabled = localStorage.getItem(STORAGE_KEY) !== "off";
  } catch (_) {}

  let probe = null;
  const getProbe = () => {
    if (probe) return probe;
    probe = document.createElement("label");
    probe.setAttribute("aria-hidden", "true");
    probe.className = "visually-hidden";
    const input = document.createElement("input");
    input.type = "checkbox";
    input.setAttribute("switch", "");
    input.tabIndex = -1;
    probe.appendChild(input);
    document.body.appendChild(probe);
    return probe;
  };

  let lastAt = 0;

  const play = (name = "light") => {
    if (!enabled) return;

    const now = performance.now();
    if (now - lastAt < 40) return;
    lastAt = now;

    if (telegramHaptic(name)) return;

    const pattern = PATTERNS[name] ?? PATTERNS.light;

    if (canVibrate) {
      navigator.vibrate(pattern);
      return;
    }

    if (canSwitchHaptic) {
      const steps = Array.isArray(pattern) ? pattern : [pattern];
      let offset = 0;
      steps.forEach((ms, index) => {
        if (index % 2 === 0) {
          if (offset === 0) getProbe().click();
          else setTimeout(() => getProbe().click(), offset);
        }
        offset += ms;
      });
    }
  };

  window.haptics = {
    play,
    selection: () => play("selection"),
    light: () => play("light"),
    medium: () => play("medium"),
    heavy: () => play("heavy"),
    success: () => play("success"),
    warning: () => play("warning"),
    error: () => play("error"),
    get enabled() {
      return enabled;
    },
    set enabled(value) {
      enabled = Boolean(value);
      try {
        localStorage.setItem(STORAGE_KEY, enabled ? "on" : "off");
      } catch (_) {}
    },
  };

  document.addEventListener(
    "click",
    (event) => {
      const target = event.target instanceof Element ? event.target : null;
      if (!target) return;

      const override = target.closest("[data-haptic]");
      if (override) {
        if (override.dataset.haptic !== "none") play(override.dataset.haptic);
        return;
      }

      for (const [selector, kind] of TAP_MAP) {
        const el = target.closest(selector);
        if (!el) continue;
        if (el.disabled || el.getAttribute("aria-disabled") === "true") return;
        play(kind);
        return;
      }
    },
    true,
  );
})();
