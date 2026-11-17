/* Exact originals only: presentation shortening must never affect copying. */
(() => {
  const status = document.createElement('div');
  status.className = 'prism-copy-status';
  status.setAttribute('role', 'status');
  status.setAttribute('aria-live', 'polite');
  status.setAttribute('aria-atomic', 'true');
  document.body.append(status);

  document.addEventListener('click', async (event) => {
    const button = event.target.closest('button[data-copy-value]');
    if (!button || button.disabled) return;
    const value = button.dataset.copyValue;
    // Clear first so a repeated outcome is announced as well.
    status.textContent = '';
    try {
      await navigator.clipboard.writeText(value);
      if (button.nextElementSibling?.classList.contains('prism-copy-fallback')) {
        button.nextElementSibling.remove();
      }
      status.textContent = 'Copied to clipboard.';
    } catch {
      let fallback = button.nextElementSibling;
      if (!fallback?.classList.contains('prism-copy-fallback')) {
        fallback = document.createElement('label');
        fallback.className = 'prism-copy-fallback';
        fallback.textContent = 'Select and copy manually';
        const field = document.createElement('textarea');
        field.readOnly = true;
        field.rows = 2;
        fallback.append(field);
        button.after(fallback);
      }
      fallback.querySelector('textarea').value = value;
      status.textContent = 'Copy unavailable. Select the value beside the button and copy it manually.';
    }
  });
})();
