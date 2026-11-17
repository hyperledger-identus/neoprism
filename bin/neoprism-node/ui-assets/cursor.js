// Cursor UI observations only. A received watch snapshot says nothing about source health.
(() => {
  const received = (card) => {
    if (!card) return;
    const present = card.dataset.cursorPresent === "true";
    const receipt = card.querySelector("[data-cursor-receipt]");
    const status = card.querySelector("[data-cursor-status]");
    const now = new Date().toISOString();
    receipt.textContent = present
      ? `This browser last successfully received the cursor snapshot at ${now} · checks every 2 seconds while responses succeed.`
      : `This browser last successfully checked for sync information at ${now}. No cursor was supplied · checks every 2 seconds while responses succeed.`;
    status.textContent = present ? "Cursor received" : "Waiting for sync information";
    card.removeAttribute("aria-busy");
  };

  const requestCard = (event) => event.detail.elt?.id === "dlt-cursor-card"
    ? document.getElementById("dlt-cursor-card") : null;

  document.addEventListener("htmx:beforeRequest", (event) => {
    const card = requestCard(event);
    if (!card) return;
    card.setAttribute("aria-busy", "true");
    card.querySelector("[data-cursor-status]").textContent = "Checking for sync information…";
  });
  document.addEventListener("htmx:afterRequest", (event) => {
    const card = requestCard(event);
    if (!card || !event.detail.failed) return;
    card.removeAttribute("aria-busy");
    card.querySelector("[data-cursor-status]").textContent = card.dataset.cursorPresent === "true"
      ? "Couldn’t refresh · showing last received cursor"
      : "Couldn’t refresh sync information · no cursor received";
  });

  received(document.getElementById("dlt-cursor-card"));
  document.addEventListener("htmx:afterSwap", (event) => {
    if (event.detail.target?.id === "dlt-cursor-card") {
      received(document.getElementById("dlt-cursor-card"));
    }
  });
})();
