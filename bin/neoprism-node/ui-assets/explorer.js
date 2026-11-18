/* Explorer delivery only. Receipt of a snapshot is not evidence of source health. */
(() => {
  // Reconcile evidence in place, retaining disclosure nodes and unchanged keyed rows.
  // Never mutate evidence while the reader is focusing, selecting or pressing in it.
  const key = node => node.nodeType === Node.ELEMENT_NODE
    ? node.id || node.dataset.refreshKey : null;
  const reconcile = (current, next) => {
    if (current.isEqualNode(next)) return;
    if (current.nodeType !== next.nodeType || current.nodeName !== next.nodeName) {
      current.replaceWith(next.cloneNode(true));
      return;
    }
    if (current.nodeType === Node.TEXT_NODE) {
      current.nodeValue = next.nodeValue;
      return;
    }
    for (const attr of [...current.attributes]) {
      if (attr.name !== 'open' && attr.name !== 'aria-expanded' && !next.hasAttribute(attr.name)) current.removeAttribute(attr.name);
    }
    for (const attr of next.attributes) {
      if (attr.name !== 'open' && attr.name !== 'aria-expanded' && current.getAttribute(attr.name) !== attr.value) current.setAttribute(attr.name, attr.value);
    }
    [...next.childNodes].forEach((child, index) => {
      let existing = current.childNodes[index];
      if (key(child)) {
        const matched = [...current.childNodes].find(candidate => key(candidate) === key(child));
        if (matched && matched !== existing) current.insertBefore(matched, existing || null);
        existing = matched;
      }
      if (!existing) current.insertBefore(child.cloneNode(true), current.childNodes[index] || null);
      else reconcile(existing, child);
    });
    while (current.childNodes.length > next.childNodes.length) current.lastChild.remove();
  };
  const selecting = node => {
    const selection = getSelection();
    return selection && !selection.isCollapsed && [...Array(selection.rangeCount).keys()]
      .some(index => selection.getRangeAt(index).intersectsNode(node));
  };
  // Receipt age is not a live region; unchanged status text must not be re-announced.
  const setText = (node, value) => {
    if (node.textContent === value) return false;
    if (selecting(node)) return true;
    node.textContent = value;
    return false;
  };
  const resources = [];
  for (const element of document.querySelectorAll('[data-refresh-resource]')) {
    const cursor = element.dataset.refreshResource === 'cursor';
    const seconds = Number(element.dataset.refreshInterval) / 1000;
    const content = element.querySelector('[data-refresh-content]');
    const status = element.querySelector('[data-cursor-status], [data-refresh-status]');
    const receipt = element.querySelector('[data-refresh-receipt]');
    const action = element.querySelector('[data-refresh-action]');
    let inflight = false;
    let generation = 0;
    let active = true;
    let controller;
    let timer;
    let refreshOnReturn = false;
    let state = element.dataset.refreshState;
    let present = element.dataset.cursorPresent === 'true';
    let lastReceipt = state === 'received' ? Date.now() : null;
    let receivedPresent = present;
    let displayed = content.innerHTML;
    let staged = null;
    let pressing = false;
    const deferred = element.querySelector('[data-refresh-deferred]');
    const interacting = () => {
      return pressing || content.contains(document.activeElement) || selecting(content);
    };
    const flush = () => {
      if (!staged || !active || document.hidden || interacting()) return;
      reconcile(content, staged.querySelector('[data-refresh-content]'));
      displayed = staged.querySelector('[data-refresh-content]').innerHTML;
      present = staged.dataset.cursorPresent === 'true';
      element.dataset.cursorPresent = String(present);
      staged = null;
      render();
    };

    const render = () => {
      let statusText;
      let receiptText;
      element.dataset.refreshState = state;
      if (state === 'pending') {
        statusText = cursor ? 'Checking for sync information…' : 'Refreshing indexed DIDs…';
      } else if (state === 'failed') {
        if (staged) {
          statusText = `Couldn’t refresh · received ${cursor ? 'sync information' : 'DID'} update waiting`;
        } else {
          statusText = cursor
            ? (present ? 'Couldn’t refresh · showing last received cursor' : 'Couldn’t refresh sync information · no cursor received')
            : (lastReceipt === null ? 'Couldn’t load indexed DIDs · no response received' : 'Couldn’t refresh · showing last received indexed DIDs');
        }
      } else {
        statusText = staged ? 'Response received · display update waiting'
          : (cursor ? (present ? 'Cursor received' : 'Waiting for sync information') : 'Indexed DID response received');
      }
      const cadence = `${state === 'failed' ? 'Retries' : 'Checks'} every ${seconds} seconds while visible.`;
      if (lastReceipt === null) {
        receiptText = `No successful response received by this browser. ${cadence}`;
      } else {
        const kind = cursor ? (receivedPresent ? 'received the cursor snapshot' : 'checked for sync information') : 'received the indexed DID response';
        receiptText = `This browser last successfully ${kind} at ${new Date(lastReceipt).toISOString()} · ${Math.max(0, Math.floor((Date.now() - lastReceipt) / 1000))} seconds ago. ${cursor && !receivedPresent ? 'No cursor was supplied. ' : ''}${cadence}`;
      }
      const metadataHeld = [
        setText(status, statusText), setText(receipt, receiptText),
        setText(action, `${state === 'failed' ? 'Retry' : 'Refresh'} ${cursor ? 'sync information' : 'indexed DIDs'}`),
      ].some(Boolean);
      deferred.hidden = !staged && !metadataHeld;
      setText(deferred, [
        staged ? 'A newer response is waiting. Displayed values stay unchanged while you focus or select here; move focus and clear the selection to show the update.' : '',
        metadataHeld ? `${statusText}. Updated browser feedback is waiting until you clear your selection.` : '',
      ].filter(Boolean).join(' '));
      action.setAttribute('aria-disabled', String(inflight));
    };

    // Event/timer dispatch is intentionally fire-and-forget; this function handles
    // request failures and owns response state, cancellation, and cleanup.
    const request = async () => {
      if (inflight || !active || document.hidden) return;
      const version = generation;
      const page = location.pathname + location.search;
      const url = element.dataset.refreshUrl;
      const current = () => active && version === generation && element.isConnected
        && page === location.pathname + location.search && url === element.dataset.refreshUrl;
      const previousState = state;
      inflight = true;
      refreshOnReturn = false;
      controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 15000);
      state = 'pending';
      element.setAttribute('aria-busy', 'true');
      render();
      try {
        const response = await fetch(url, {cache: 'no-store', signal: controller.signal});
        if (!response.ok) throw new Error('request failed');
        const doc = new DOMParser().parseFromString(await response.text(), 'text/html');
        if (!current()) return;
        if (controller.signal.aborted) throw new Error('request timed out');
        const next = doc.getElementById(element.id);
        const nextContent = next?.querySelector('[data-refresh-content]');
        if (!nextContent || next.dataset.refreshState !== 'received') throw new Error('invalid response');
        receivedPresent = next.dataset.cursorPresent === 'true';
        staged = nextContent.innerHTML === displayed ? null : next;
        lastReceipt = Date.now();
        state = 'received';
        flush();
      } catch {
        // Never render a failed response body or turn a failed request into an empty dataset.
        if (current()) state = 'failed';
      } finally {
        clearTimeout(timeout);
        inflight = false;
        element.removeAttribute('aria-busy');
        if (!current()) state = previousState;
        if (active) render();
        if (active && refreshOnReturn && !document.hidden) {
          refreshOnReturn = false;
          void request();
        }
      }
    };
    const arm = () => { timer = setTimeout(tick, seconds * 1000); };
    const tick = () => {
      if (document.hidden) return;
      arm();
      void request();
    };
    action.addEventListener('click', (event) => {
      if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
      event.preventDefault();
      void request();
    });
    document.addEventListener('visibilitychange', () => {
      clearTimeout(timer);
      if (document.hidden) {
        generation++;
        controller?.abort();
      } else {
        if (inflight) refreshOnReturn = true;
        else void request();
        arm();
      }
    });
    window.addEventListener('pagehide', () => {
      active = false;
      generation++;
      controller?.abort();
      clearTimeout(timer);
    });
    window.addEventListener('pageshow', (event) => {
      if (!event.persisted) return;
      active = true;
      if (inflight) refreshOnReturn = true;
      else void request();
      if (!document.hidden) arm();
    });
    content.addEventListener('focusout', () => queueMicrotask(flush));
    document.addEventListener('selectionchange', () => { flush(); render(); });
    content.addEventListener('pointerdown', () => { pressing = true; });
    document.addEventListener('pointerup', () => { pressing = false; queueMicrotask(flush); });
    document.addEventListener('pointercancel', () => { pressing = false; queueMicrotask(flush); });
    element.addEventListener('toggle', (event) => {
      if (event.target.matches('details')) event.target.querySelector('summary')?.setAttribute('aria-expanded', String(event.target.open));
    }, true);
    render();
    resources.push(render);
    if (!document.hidden) arm();
  }
  const age = () => {
    if (!document.hidden) resources.forEach(render => render());
    setTimeout(age, 1000);
  };
  if (resources.length) setTimeout(age, 1000);
})();
