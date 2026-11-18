/* Exact originals only: presentation shortening must never affect copying. */
(() => {
  document.documentElement.classList.add('prism-copy-ready');
  const status = document.createElement('div');
  status.className = 'prism-copy-status';
  status.setAttribute('role', 'status');
  status.setAttribute('aria-live', 'polite');
  status.setAttribute('aria-atomic', 'true');
  document.body.append(status);

  document.addEventListener('click', async (event) => {
    const button = event.target.closest('button[data-copy-value]');
    if (!button || button.disabled) return;
    const value = button.dataset.copyEncoding === 'json'
      ? JSON.parse(button.dataset.copyValue) : button.dataset.copyValue;
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
        const exact = /[\r\0]/.test(value);
        const field = document.createElement(exact ? 'pre' : 'textarea');
        if (exact) {
          // Textarea.value normalizes CR even after lossless transport decoding.
          // A text node retains it; handle select-all locally and use the native
          // copy event (not the permission-gated async API) for the exact value.
          field.tabIndex = 0;
          field.setAttribute('role', 'textbox');
          field.setAttribute('aria-label', 'Select and copy manually');
          field.setAttribute('aria-readonly', 'true');
          field.setAttribute('aria-multiline', 'true');
          field.addEventListener('keydown', event => {
            if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'a') {
              event.preventDefault();
              const range = document.createRange();
              range.selectNodeContents(field.firstChild);
              const selection = window.getSelection();
              selection.removeAllRanges();
              selection.addRange(range);
            }
          });
          field.addEventListener('copy', event => {
            const selection = window.getSelection();
            if (field.contains(selection.anchorNode)
                && field.contains(selection.focusNode)
                && selection.toString() === value
                && event.clipboardData) {
              event.preventDefault();
              event.clipboardData.setData('text/plain', value);
            }
          });
        } else {
          field.readOnly = true;
          field.rows = 2;
        }
        fallback.append(field);
        button.parentElement.classList.add('prism-copy-container');
        button.after(fallback);
      }
      const field = fallback.lastElementChild;
      if (field.tagName === 'PRE') field.textContent = value;
      else field.value = value;
      status.textContent = 'Couldn’t copy. Select the value beside the button to copy manually.';
    }
  });
})();
