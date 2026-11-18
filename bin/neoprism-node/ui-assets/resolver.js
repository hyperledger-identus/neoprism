(() => {
  const form = document.querySelector(".prism-resolve-form");
  const input = form?.querySelector("#did");
  const button = form?.querySelector('button[type="submit"]');
  const status = document.getElementById("resolution-status");
  const pending = document.getElementById("resolution-pending");
  const result = document.getElementById("resolution-result");
  const recovery = document.getElementById("resolution-recovery");
  const retry = document.getElementById("resolution-retry");
  const previous = document.getElementById("resolution-previous");
  if (!form || !input || !button || !status || !pending || !result || !recovery || !retry || !previous || !window.fetch) return;

  let request = null;
  let resolvedUrl = location.pathname + location.search;
  const ready = () => {
    button.disabled = false;
    button.textContent = "Resolve";
    input.readOnly = false;
    form.removeAttribute("aria-busy");
    pending.hidden = true;
    result.hidden = false;
    document.querySelectorAll("[data-resolver-retry]").forEach(link => link.removeAttribute("aria-disabled"));
  };
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (request) return;
    const submittedFrom = document.activeElement;
    const url = new URL(form.action);
    url.search = new URLSearchParams(new FormData(form)).toString();
    request = new AbortController();
    const controller = request;
    const timeout = setTimeout(() => controller.abort(), 30000);
    button.disabled = true;
    button.textContent = "Resolving…";
    input.readOnly = true;
    form.setAttribute("aria-busy", "true");
    status.textContent = "Resolving identity…";
    result.hidden = true;
    pending.hidden = false;
    document.querySelectorAll("[data-resolver-retry]").forEach(link => link.setAttribute("aria-disabled", "true"));
    try {
      const response = await fetch(url, { signal: controller.signal, headers: { Accept: "text/html" } });
      if (!response.ok) throw new Error("request failed");
      const page = new DOMParser().parseFromString(await response.text(), "text/html");
      const next = page.getElementById("resolution-result");
      const nextInput = page.getElementById("did");
      if (controller !== request) return;
      if (!next || !nextInput) throw new Error("invalid response");
      const announcement = (next.querySelector(".prism-identity-status") || next.querySelector("h2"))?.textContent || "Resolution complete.";
      const recoveryHadFocus = recovery.contains(document.activeElement);
      result.replaceChildren(...next.childNodes);
      result.dataset.resolvedInput = next.dataset.resolvedInput;
      const evidence = result.querySelector("#ledger-evidence");
      if (evidence && window.matchMedia("(min-width: 1280px)").matches) evidence.open = true;
      recovery.hidden = true;
      if (recoveryHadFocus) input.focus({ preventScroll: true });
      if (nextInput.getAttribute("aria-invalid") === "true") input.setAttribute("aria-invalid", "true");
      else input.removeAttribute("aria-invalid");
      history.pushState(null, "", url);
      resolvedUrl = location.pathname + location.search;
      status.textContent = announcement;
    } catch {
      if (controller !== request) return;
      retry.href = url.href;
      previous.hidden = !result.hasAttribute("data-resolved-input");
      recovery.hidden = false;
      status.textContent = "Couldn’t load this resolution. Your input is preserved; retry is available.";
    } finally {
      clearTimeout(timeout);
      if (controller === request) {
        request = null;
        ready();
        // Disabling a focused native submit button can blur it. Restore only
        // that control, never override focus the reader moved elsewhere.
        if (submittedFrom === button && document.activeElement === document.body) {
          button.focus({ preventScroll: true });
        }
      }
    }
  });
  document.addEventListener("click", (event) => {
    const link = event.target.closest("[data-resolver-retry]");
    if (!link || event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    if (request) return;
    input.value = new URL(link.href).searchParams.get("did") || "";
    // If the result containing this link is replaced, return focus to the input.
    if (result.contains(document.activeElement)) input.focus({ preventScroll: true });
    form.requestSubmit();
  });
  window.addEventListener("pageshow", (event) => {
    if (event.persisted) {
      request?.abort();
      request = null;
      status.textContent = "";
    }
    if (!request) ready();
  });
  // DID-query history must resolve that URL, but section-only navigation keeps
  // the current dossier and its open inspectors intact.
  window.addEventListener("popstate", () => {
    if (location.pathname + location.search !== resolvedUrl) location.reload();
  });
})();
