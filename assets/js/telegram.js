(() => {
  const tg = window.Telegram && window.Telegram.WebApp;
  const active = Boolean(tg && tg.initData);

  const attempt = (fn) => {
    try {
      fn();
    } catch (_) {}
  };

  let backHandler = null;

  window.tgApp = {
    active,

    startParam() {
      if (!active || !tg.initDataUnsafe) return null;
      return tg.initDataUnsafe.start_param || null;
    },

    setBack(handler) {
      if (!active || !tg.BackButton) return;
      if (backHandler) attempt(() => tg.BackButton.offClick(backHandler));
      backHandler = handler;
      if (handler) {
        attempt(() => tg.BackButton.onClick(handler));
        attempt(() => tg.BackButton.show());
      } else {
        attempt(() => tg.BackButton.hide());
      }
    },

    openLink(url) {
      if (!active || typeof tg.openLink !== "function") return false;
      tg.openLink(url);
      return true;
    },
  };

  if (!active) return;

  document.documentElement.classList.add("is-telegram");

  attempt(() => tg.ready());
  attempt(() => tg.expand());
  attempt(() => tg.disableVerticalSwipes && tg.disableVerticalSwipes());

  const surface = getComputedStyle(document.documentElement)
    .getPropertyValue("--surface-1")
    .trim();
  if (surface) {
    attempt(() => tg.setHeaderColor && tg.setHeaderColor(surface));
    attempt(() => tg.setBackgroundColor && tg.setBackgroundColor(surface));
    attempt(() => tg.setBottomBarColor && tg.setBottomBarColor(surface));
  }

  document.addEventListener("click", (event) => {
    const link =
      event.target instanceof Element
        ? event.target.closest("a[data-external]")
        : null;
    if (link && window.tgApp.openLink(link.href)) event.preventDefault();
  });
})();
