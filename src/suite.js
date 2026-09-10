(() => {
  const trigger = document.querySelector('#suite-trigger');
  const menu = document.querySelector('#suite-menu');
  const error = document.querySelector('#suite-error');
  const links = Object.freeze({
    hub: 'https://github.com/BOBWORKS-XR/CREATOR-PROJECT-SETUP/blob/master/docs/CREATOR-HUB-PLAN.md',
    mcp: 'https://github.com/BOBWORKS-XR/CREATOR-WORKS-UNITY-MCP/releases',
  });
  const items = () => [...menu.querySelectorAll('button:not(:disabled)')];
  const isOpen = () => trigger.getAttribute('aria-expanded') === 'true';
  function close(restoreFocus = false) {
    menu.classList.add('hidden');
    trigger.setAttribute('aria-expanded', 'false');
    trigger.setAttribute('aria-label', 'Open Creator apps');
    if (restoreFocus) trigger.focus();
  }
  trigger.addEventListener('click', () => {
    if (isOpen()) return close(true);
    menu.classList.remove('hidden');
    trigger.setAttribute('aria-expanded', 'true');
    trigger.setAttribute('aria-label', 'Close Creator apps');
    menu.querySelector('button.suite-item')?.focus();
  });
  document.querySelector('#suite-close').addEventListener('click', () => close(true));
  document.querySelector('#suite-current').addEventListener('click', () => close(true));
  document.addEventListener('keydown', event => {
    if (isOpen() && event.key === 'Escape') { event.preventDefault(); close(true); }
  });
  document.addEventListener('pointerdown', event => {
    if (isOpen() && !menu.contains(event.target) && !trigger.contains(event.target)) close();
  });
  document.addEventListener('focusin', event => {
    if (isOpen() && !menu.contains(event.target) && !trigger.contains(event.target)) close();
  });
  menu.addEventListener('keydown', event => {
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const buttons = items();
    const current = buttons.indexOf(document.activeElement);
    const index = event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1
      : (current + (event.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length;
    buttons[index]?.focus();
  });
  for (const button of menu.querySelectorAll('[data-suite-link]')) {
    button.addEventListener('click', async () => {
      error.classList.add('hidden');
      try {
        await window.__TAURI__.core.invoke('open_official_url', { url: links[button.dataset.suiteLink] });
      } catch (reason) {
        error.textContent = `Could not open the link: ${String(reason)}`;
        error.classList.remove('hidden');
      }
    });
  }
})();
