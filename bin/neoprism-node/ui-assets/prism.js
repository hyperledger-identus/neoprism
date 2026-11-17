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

  const form = document.querySelector(".prism-resolve-form");
  const button = form?.querySelector('button[type="submit"]');
  const status = document.getElementById("resolution-status");
  if (form && button && status) {
    const ready = () => {
      button.disabled = false;
      button.textContent = "Resolve";
      form.removeAttribute("aria-busy");
      status.textContent = "";
    };
    // Native input handles Enter, required validation and IME composition.
    // Never rewrite its value or intercept its keystrokes.
    form.addEventListener("submit", (event) => {
      if (button.disabled) {
        event.preventDefault();
        return;
      }
      button.disabled = true;
      button.textContent = "Resolving…";
      form.setAttribute("aria-busy", "true");
      status.textContent = "Resolving identity…";
    });
    window.addEventListener("pageshow", ready);
  }
})();
