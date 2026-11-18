(() => {
  const menu = document.getElementById("mobile-menu");
  const toggle = document.getElementById("menu-toggle");
  if (menu && toggle) {
    const update = () => toggle.setAttribute("aria-expanded", String(menu.open));
    update();
    menu.addEventListener("toggle", update);
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && menu.open) {
        event.preventDefault();
        menu.open = false;
        update();
        toggle.focus();
      }
    });
  }

  // Native details remains usable without scripts. Only choose the initial
  // presentation here; never override a reader's expansion on later resizing.
  const evidence = document.getElementById("ledger-evidence");
  if (evidence && window.matchMedia("(min-width: 1280px)").matches) {
    evidence.open = true;
  }

})();
