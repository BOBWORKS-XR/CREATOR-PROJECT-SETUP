const { test, expect } = require('@playwright/test');

async function setup(page, options = {}) {
  await page.addInitScript(options => {
    window.calls = [];
    window.options = options;
    window.__TAURI__ = { event: { listen: async (name, callback) => {
      window.progressCallback = callback;
      return () => { window.progressCallback = null; };
    } }, core: { invoke: async (command, args) => {
      window.calls.push({ command, args });
      if (command === 'probe_environment') return {
        platform: window.options.platform || 'windows', ready: true, hubInstalled: true, unityCliInstalled: false,
        hubVersion: window.options.legacyHub ? '3.14.4' : '3.21.1', hubAutoRegistration: !window.options.legacyHub,
        suggestedProjectParent: 'F:\\UnityTest', blockers: [],
        recipe: { editorVersion: '6000.3.21f1', creatorSdkVersion: '4.0.14', urpVersion: '17.3.0', inputSystemVersion: '1.20.0' },
        editors: [{ exactRecipe: true, androidPlayer: true, androidSdk: true, androidNdk: true, openJdk: true, windowsStandalone: true, urpTemplate: 'template.tgz' }],
      };
      if (command === 'create_project') {
        if (window.options.pending) await new Promise(resolve => window.finishCreate = resolve);
        if (window.options.failCreate) throw 'Project already exists. No files were changed.';
        return { success: true, projectPath: `${args.request.parentDirectory}\\${args.request.projectName}`, message: 'Project validated.', hub: window.options.legacyHub ? { registered: false, requiresHubUpdate: true, message: 'Unity Hub 3.14.4 uses the old registry. Update to 3.21.1 or use Add > Add project from disk.' } : { registered: !window.options.failHub, refreshPending: !window.options.failHub, message: window.options.failHub ? 'Download failed. Your project is ready to open.' : 'Project registered. A running Unity Hub may need a full restart to show it in Projects.' } };
      }
      if (command === 'register_project') return { registered: true, refreshPending: true, message: 'Project registered. A running Unity Hub may need a full restart to show it in Projects.' };
      if (command === 'restart_hub') {
        if (window.options.pendingRestart) await new Promise(resolve => window.finishRestart = resolve);
        if (window.options.failRestart) throw 'Unity Hub did not fully exit. Nothing was force-closed.';
        return !window.options.cancelRestart;
      }
      if (command === 'inspect_project') return {
        projectPath: args.path, fingerprint: 'reviewed-files-sha256', canRepair: !window.options.blocked, canValidate: !window.options.blocked,
        findings: [{ status: window.options.blocked ? 'blocked' : 'repair', title: 'Visual Scripting setup', detail: window.options.blocked ? 'Project is open. Close Unity.' : 'Node database is missing.' }, { status: 'pass', title: 'Unity version', detail: '6000.3.21f1' }],
        proposedChanges: window.options.blocked ? [] : ['Rebuild Creator Visual Scripting nodes; retain existing type selections.'],
      };
      if (command === 'run_existing_project') {
        if (window.options.pendingRepair) await new Promise(resolve => window.finishRepair = resolve);
        return { success: !window.options.failRepair, projectPath: args.request.projectPath, backupPath: `${args.request.projectPath}\\.creator-project-setup\\backups\\test`, reportPath: 'result.json', message: window.options.failRepair ? 'Compilation failed. Backup retained. No automatic rollback was attempted.' : 'Unity validation passed.' };
      }
      if (command === 'open_project' && window.options.failOpen) throw 'Unity could not be opened.';
    } } };
  }, options);
  await page.goto('http://127.0.0.1:4187');
  await expect(page.locator('#create-button')).toBeEnabled();
}

test('Hub failure preserves completed project and retry does not recreate it', async ({ page }) => {
  await setup(page, { failHub: true });
  await page.locator('#create-button').click();
  await expect(page.locator('#hub-result')).toContainText('Download failed');
  await expect(page.locator('#open-project-button')).toBeEnabled();
  await page.locator('#retry-hub-button').click();
  await expect(page.locator('#hub-result')).toContainText('Project registered.');
  await expect(page.locator('#retry-hub-button')).toBeHidden();
  const calls = await page.evaluate(() => window.calls);
  expect(calls.filter(call => call.command === 'create_project')).toHaveLength(1);
  expect(calls.filter(call => call.command === 'register_project')).toHaveLength(1);
});

test('legacy Hub does not receive a green registration claim and update retry preserves project', async ({ page }, testInfo) => {
  await setup(page, { legacyHub: true });
  await expect(page.locator('#requirements')).toContainText('Hub 3.14.4');
  await expect(page.locator('#requirements')).toContainText('Hub 3.21.1+ required');
  await page.locator('#create-button').click();
  await expect(page.locator('#hub-result')).toHaveClass('hub-result pending');
  await expect(page.locator('#hub-result')).toContainText('3.14.4');
  await expect(page.locator('#open-project-button')).toBeEnabled();
  await expect(page.locator('#retry-hub-button')).toHaveText('Recheck Hub and add project');
  await page.screenshot({ path: testInfo.outputPath('legacy-hub.png'), fullPage: true });
  await page.locator('#open-result-hub-button').click();
  expect(await page.evaluate(() => window.calls.filter(call => call.command === 'launch_hub').length)).toBe(1);
  await page.evaluate(() => window.options.legacyHub = false);
  await page.locator('#retry-hub-button').click();
  await expect(page.locator('#hub-result')).toHaveClass('hub-result pending');
  await expect(page.locator('#requirements')).toContainText('Hub 3.21.1');
  expect(await page.evaluate(() => window.calls.filter(call => call.command === 'create_project').length)).toBe(1);
});

async function inspectExisting(page, options = {}) {
  await setup(page, options);
  await page.locator('#existing-mode').click();
  await page.locator('#existing-path').fill('F:\\UnityTest\\Existing project');
  await page.locator('#inspect-button').click();
  await expect(page.locator('#inspection-findings')).toContainText('Visual Scripting');
}

test('registered projects offer a confirmed Hub restart without claiming UI visibility', async ({ page }, testInfo) => {
  await setup(page);
  await page.locator('#create-button').click();
  await expect(page.locator('#hub-result')).toHaveClass('hub-result pending');
  await expect(page.locator('#restart-hub-button')).toBeVisible();
  expect(await page.evaluate(() => window.calls.some(call => call.command === 'restart_hub'))).toBe(false);
  await page.screenshot({ path: testInfo.outputPath('hub-restart-pending.png'), fullPage: true });
  await page.locator('#restart-hub-button').click();
  await expect(page.locator('#hub-result')).toHaveClass('hub-result');
  await expect(page.locator('#hub-result')).toContainText('Unity Hub restarted. Check its Projects list.');
  await expect(page.locator('#restart-hub-button')).toBeHidden();
  await expect(page.locator('#open-project-button')).toBeEnabled();
  const calls = await page.evaluate(() => window.calls);
  expect(calls.filter(call => call.command === 'create_project')).toHaveLength(1);
  expect(calls.filter(call => call.command === 'restart_hub')).toHaveLength(1);
  expect(calls.filter(call => call.command === 'open_project')).toHaveLength(0);
  await page.locator('#create-another-button').click();
  await expect(page.locator('#restart-hub-button')).toBeHidden();
});

test('cancelling the native restart prompt preserves the pending state', async ({ page }) => {
  await setup(page, { cancelRestart: true });
  await page.locator('#create-button').click();
  const before = await page.locator('#hub-result').textContent();
  await page.locator('#restart-hub-button').click();
  await expect(page.locator('#hub-result')).toHaveText(before);
  await expect(page.locator('#restart-hub-button')).toBeEnabled();
  await expect(page.locator('#open-project-button')).toBeEnabled();
});

test('restart failures retain the project and allow retry', async ({ page }) => {
  await setup(page, { failRestart: true });
  await page.locator('#create-button').click();
  await page.locator('#restart-hub-button').click();
  await expect(page.locator('#action-error')).toContainText('Nothing was force-closed');
  await expect(page.locator('#hub-result')).toHaveClass('hub-result pending');
  await expect(page.locator('#restart-hub-button')).toBeEnabled();
  await expect(page.locator('#open-result-hub-button')).toBeEnabled();
  await page.evaluate(() => window.options.failRestart = false);
  await page.locator('#restart-hub-button').click();
  await expect(page.locator('#action-error')).toBeHidden();
  await expect(page.locator('#hub-result')).toContainText('Unity Hub restarted');
  expect(await page.evaluate(() => window.calls.filter(call => call.command === 'create_project').length)).toBe(1);
});

test('a pending restart locks conflicting actions and cannot be submitted twice', async ({ page }) => {
  await setup(page, { pendingRestart: true });
  await page.locator('#create-button').click();
  await page.locator('#restart-hub-button').click();
  await expect(page.locator('#restart-hub-button')).toBeDisabled();
  await expect(page.locator('#existing-mode')).toBeDisabled();
  await expect(page.locator('#open-project-button')).toBeDisabled();
  await expect(page.locator('#create-another-button')).toBeDisabled();
  await expect(page.locator('#refresh-button')).toBeDisabled();
  await page.locator('#restart-hub-button').dispatchEvent('click');
  expect(await page.evaluate(() => window.calls.filter(call => call.command === 'restart_hub').length)).toBe(1);
  await page.evaluate(() => window.finishRestart());
  await expect(page.locator('#open-project-button')).toBeEnabled();
});

test('unsupported platforms retain manual Hub refresh instructions', async ({ page }) => {
  await setup(page, { platform: 'macos' });
  await page.locator('#create-button').click();
  await expect(page.locator('#restart-hub-button')).toBeHidden();
  await expect(page.locator('#hub-result')).toContainText('Fully quit Hub, then choose Open Unity Hub.');
});

test('existing inspection is separate from approval and invalidates on path changes', async ({ page }) => {
  await inspectExisting(page);
  await expect(page.locator('#repair-button')).toBeDisabled();
  await page.locator('#repair-button').dispatchEvent('click');
  expect(await page.evaluate(() => window.calls.some(call => call.command === 'run_existing_project'))).toBe(false);
  await page.locator('#existing-approval').check();
  await expect(page.locator('#repair-button')).toBeEnabled();
  await page.locator('#existing-path').fill('F:\\UnityTest\\Different project');
  await expect(page.locator('#inspection-results')).toBeHidden();
  await expect(page.locator('#existing-approval')).not.toBeChecked();
  await page.locator('#repair-button').dispatchEvent('click');
  expect(await page.evaluate(() => window.calls.some(call => call.command === 'run_existing_project'))).toBe(false);
});

test('approved repair sends exact reviewed path and fingerprint, locks controls and retains report', async ({ page }) => {
  await inspectExisting(page, { pendingRepair: true });
  await page.locator('#existing-approval').check();
  await page.locator('#repair-button').click();
  await expect(page.locator('#new-mode')).toBeDisabled();
  await expect(page.locator('#existing-path')).toBeDisabled();
  await page.locator('#repair-button').dispatchEvent('click');
  await page.evaluate(() => window.progressCallback({ payload: { step: 4, detail: 'Generating Creator node database.' } }));
  await expect(page.locator('#existing-detail')).toHaveText('Generating Creator node database.');
  await page.evaluate(() => window.finishRepair());
  await expect(page.locator('#existing-result')).toContainText('Validation passed');
  await expect(page.locator('#existing-result')).toContainText('Backup:');
  await page.locator('#open-existing-button').click();
  const calls = await page.evaluate(() => window.calls);
  expect(calls.filter(call => call.command === 'run_existing_project')).toEqual([{ command: 'run_existing_project', args: { request: { projectPath: 'F:\\UnityTest\\Existing project', fingerprint: 'reviewed-files-sha256', repair: true, approved: true } } }]);
  expect(calls.find(call => call.command === 'open_project').args.path).toBe('F:\\UnityTest\\Existing project');
  expect(await page.evaluate(() => window.progressCallback)).toBe(null);
  await expect(page.locator('#repair-button')).toBeDisabled();
});

test('validation failure retains backup information and does not offer a successful Open', async ({ page }) => {
  await inspectExisting(page, { failRepair: true });
  await page.locator('#existing-approval').check();
  await page.locator('#validate-button').click();
  await expect(page.locator('#existing-result')).toContainText('Compilation failed');
  await expect(page.locator('#existing-result')).toContainText('Backup:');
  await expect(page.locator('#open-existing-button')).toBeHidden();
  const call = await page.evaluate(() => window.calls.find(call => call.command === 'run_existing_project'));
  expect(call.args.request.repair).toBe(false);
});

test('blocked projects cannot start either operation', async ({ page }) => {
  await inspectExisting(page, { blocked: true });
  await expect(page.locator('#repair-button')).toBeHidden();
  await expect(page.locator('#validate-button')).toBeHidden();
  await expect(page.locator('#existing-approval-label')).toBeHidden();
  await expect(page.locator('#inspection-findings')).toContainText('Blocked');
});

for (const width of [980, 720]) {
  test(`existing repair review layout at ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 760 });
    await inspectExisting(page);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: testInfo.outputPath('repair-review.png'), fullPage: true });
    expect(await page.locator('.suite-trigger img').evaluate(img => img.naturalWidth > 0)).toBe(true);
  });
}

test('completion cannot recreate the project, including after a refresh', async ({ page }) => {
  await setup(page);
  await page.locator('#create-button').click();
  await expect(page.locator('#open-project-button')).toBeVisible();
  await expect(page.locator('#create-button')).toBeHidden();
  await expect(page.locator('#project-name')).toBeDisabled();
  await page.locator('#refresh-button').click();
  await expect(page.locator('#overall-status')).toHaveText('Project ready');
  await page.locator('#create-button').dispatchEvent('click');
  await page.locator('#open-project-button').click();
  const calls = await page.evaluate(() => window.calls);
  expect(calls.filter(call => call.command === 'create_project')).toHaveLength(1);
  expect(calls.find(call => call.command === 'open_project').args.path).toBe('F:\\UnityTest\\My Creator Space');
});

test('pending creation rejects duplicate and refresh events', async ({ page }) => {
  await setup(page, { pending: true });
  await page.locator('#create-button').click();
  await expect(page.locator('#browse-button')).toBeDisabled();
  await expect(page.locator('#refresh-button')).toBeDisabled();
  await page.locator('#refresh-button').dispatchEvent('click');
  await page.locator('#create-button').dispatchEvent('click');
  expect(await page.evaluate(() => window.calls.filter(call => call.command === 'create_project').length)).toBe(1);
  expect(await page.evaluate(() => window.calls.filter(call => call.command === 'probe_environment').length)).toBe(1);
  await page.evaluate(() => window.finishCreate());
  await expect(page.locator('#open-project-button')).toBeVisible();
});

test('real progress stages render and unsubscribe on completion', async ({ page }, testInfo) => {
  await setup(page, { pending: true });
  await page.locator('#create-button').click();
  await page.evaluate(() => window.progressCallback({ payload: { step: 4, detail: 'Generating Creator SDK nodes.' } }));
  await expect(page.locator('#activity-title')).toHaveText('Configure Visual Scripting');
  await expect(page.locator('.progress-steps .done')).toHaveCount(3);
  await expect(page.locator('#activity-message')).toHaveText('Generating Creator SDK nodes.');
  await page.screenshot({ path: testInfo.outputPath('progress.png'), fullPage: true });
  await page.evaluate(() => window.finishCreate());
  await expect(page.locator('#open-project-button')).toBeVisible();
  expect(await page.evaluate(() => window.progressCallback)).toBe(null);
});

test('opening failure preserves success and supports retry', async ({ page }) => {
  await setup(page, { failOpen: true });
  await page.locator('#create-button').click();
  await page.locator('#open-project-button').click();
  await expect(page.getByRole('alert')).toContainText('could not be opened');
  await expect(page.locator('#result')).toHaveClass('result success');
  await expect(page.locator('#create-button')).toBeHidden();
  await page.evaluate(() => window.options.failOpen = false);
  await page.locator('#open-project-button').click();
  await expect(page.getByRole('alert')).toBeHidden();
});

test('creation failure allows retry without claiming a completed project', async ({ page }) => {
  await setup(page, { failCreate: true });
  await page.locator('#create-button').click();
  await expect(page.locator('#result')).toContainText('No files were changed');
  await expect(page.locator('#create-button')).toBeEnabled();
  await expect(page.locator('#open-project-button')).toBeHidden();
  await expect(page.locator('#project-name')).toBeEnabled();
  await expect(page.locator('#project-details')).toBeVisible();
  await expect(page.locator('#project-name')).toHaveValue('My Creator Space');
  await expect(page.locator('#project-summary')).toBeHidden();
});

test('creating another project explicitly unlocks a new name', async ({ page }) => {
  await setup(page);
  await page.locator('#create-button').click();
  await page.locator('#create-another-button').click();
  await expect(page.locator('#project-name')).toBeEmpty();
  await expect(page.locator('#project-name')).toBeFocused();
  await expect(page.locator('#project-details')).toBeVisible();
  await expect(page.locator('#project-summary')).toBeHidden();
  await expect(page.locator('#result')).toBeHidden();
  await page.locator('#project-name').fill('Second project');
  await page.locator('#create-button').click();
  await page.locator('#open-project-button').click();
  expect(await page.evaluate(() => window.calls.find(call => call.command === 'open_project').args.path)).toBe('F:\\UnityTest\\Second project');
});

for (const width of [980, 720]) {
  test(`completed layout at ${width}px with long path`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 760 });
    await setup(page);
    await page.locator('#parent-folder').fill('F:\\' + 'LongFolderName'.repeat(18));
    await page.locator('#create-button').click();
    await expect(page.locator('#result')).toHaveClass('result success');
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: testInfo.outputPath('completed.png'), fullPage: true });
  });
}

test('creation replaces fields with a summary, without changing the request', async ({ page }) => {
  await setup(page, { pending: true });
  await page.locator('#project-name').fill('Forest <test>');
  await page.locator('#create-button').click();
  await expect(page.locator('#project-details')).toBeHidden();
  await expect(page.locator('#summary-name')).toHaveText('Forest <test>');
  await expect(page.locator('#activity')).toBeVisible();
  await page.evaluate(() => window.progressCallback({ payload: { step: 4, detail: 'Compiling.' } }));
  await expect(page.locator('#stage-progress')).toHaveJSProperty('value', 3);
  await page.evaluate(() => window.progressCallback({ payload: { step: 2, detail: 'Old event.' } }));
  await expect(page.locator('#stage-progress')).toHaveJSProperty('value', 3);
  await page.evaluate(() => window.finishCreate());
  await expect(page.locator('#project-details')).toBeHidden();
  await expect(page.locator('#summary-path')).toHaveText('F:\\UnityTest\\Forest <test>');
  await expect(page.locator('#activity')).toBeHidden();
});

test('app switcher supports keyboard, outside dismissal and bounded links', async ({ page }) => {
  await setup(page);
  const trigger = page.locator('#suite-trigger');
  await expect(trigger).toHaveAttribute('aria-expanded', 'false');
  await trigger.focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('[data-suite-link="hub"]')).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(page.locator('[data-suite-link="mcp"]')).toBeFocused();
  await page.keyboard.press('Enter');
  expect(await page.evaluate(() => window.calls.at(-1))).toEqual({ command: 'open_official_url', args: { url: 'https://github.com/BOBWORKS-XR/CREATOR-WORKS-UNITY-MCP/releases' } });
  await page.keyboard.press('Escape');
  await expect(trigger).toBeFocused();
  await expect(page.locator('#suite-menu')).toBeHidden();
  await trigger.click();
  await page.locator('#suite-dismiss').click({ position: { x: 600, y: 180 } });
  await expect(page.locator('#suite-menu')).toBeHidden();
  await expect(trigger).toBeFocused();
  await trigger.click();
  await page.keyboard.press('End');
  await expect(page.locator('#suite-current')).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(page.locator('#suite-menu')).toBeHidden();
  expect(await page.evaluate(() => window.calls.every(call => ['probe_environment', 'open_official_url'].includes(call.command)))).toBe(true);
});

test('external link failures are visible and do not claim installation', async ({ page }) => {
  await setup(page);
  await page.evaluate(() => window.__TAURI__.core.invoke = async () => { throw 'Browser unavailable'; });
  await page.locator('#suite-trigger').click();
  await page.locator('[data-suite-link="hub"]').click();
  await expect(page.locator('#suite-error')).toContainText('Browser unavailable');
  await expect(page.locator('[data-suite-link="hub"]')).toContainText('In development');
  await page.keyboard.press('Escape');
  await expect(page.locator('#suite-trigger')).toBeFocused();
});

for (const width of [980, 720, 560, 390]) {
  test(`compact shell at ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 620 });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await setup(page);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect(await page.locator('.suite-trigger img').evaluate(img => img.naturalWidth)).toBe(1024);
    if (width === 980) {
      expect((await page.locator('#create-button').boundingBox()).y).toBeLessThan(350);
      expect((await page.locator('footer').boundingBox()).y).toBeLessThan(620);
    }
    await page.screenshot({ path: testInfo.outputPath('setup.png'), fullPage: true });
    await page.locator('#suite-trigger').click();
    await expect(page.locator('#suite-menu')).toBeVisible();
    await expect(page.locator('#suite-shell')).toHaveCSS('width', '224px');
    await expect(page.locator('#suite-shell')).toHaveCSS('height', '352px');
    await expect(page.locator('.app-header .title-block')).toHaveCSS('opacity', '0');
    await expect(page.locator('.suite-brand')).toHaveCSS('opacity', '1');
    await expect(page.locator('.suite-brand')).toHaveCSS('visibility', 'visible');
    expect(await page.locator('.suite-brand').evaluate(el => el.getBoundingClientRect().x)).toBe(15);
    expect(await page.locator('#suite-shell').evaluate(el => el.scrollLeft)).toBe(0);
    await page.screenshot({ path: testInfo.outputPath('switcher.png'), fullPage: true });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect(errors).toEqual([]);
  });
}

test('drawer morphs from the tab without moving page content', async ({ page }, testInfo) => {
  await setup(page);
  const bounds = await page.locator('#create-button').boundingBox();
  const closed = await page.locator('#suite-trigger').boundingBox();
  await page.locator('#suite-trigger').click();
  await expect(page.locator('#suite-shell')).toHaveCSS('width', '224px');
  await expect(page.locator('#suite-shell')).toHaveCSS('height', '352px');
  await expect(page.locator('.app-header .title-block')).toHaveCSS('opacity', '0');
  const expanded = await page.locator('#suite-trigger').boundingBox();
  expect(expanded.x - closed.x).toBe(169);
  expect(expanded.y).toBe(closed.y);
  expect(await page.locator('#create-button').boundingBox()).toEqual(bounds);
  expect(await page.locator('#suite-shell').evaluate(el => el.scrollTop)).toBe(0);
  await expect(page.locator('#suite-close')).toHaveCount(0);
  await page.screenshot({ path: testInfo.outputPath('expanded-drawer.png'), fullPage: true });
  await page.locator('#suite-trigger').click();
  await expect(page.locator('#suite-menu')).toBeHidden();
  await expect(page.locator('#suite-menu')).toHaveJSProperty('inert', true);
  await expect(page.locator('#suite-shell')).toHaveCSS('width', '55px');
  await expect(page.locator('.app-header .title-block')).toHaveCSS('opacity', '1');
  await expect(page.locator('#suite-trigger')).toBeFocused();
});

test('drawer transitions reverse and reduced motion removes animation', async ({ page }) => {
  await setup(page);
  const properties = await page.evaluate(async () => {
    document.querySelector('#suite-trigger').click();
    await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    const shell = document.querySelector('#suite-shell');
    const transitions = shell.getAnimations().map(animation => animation.transitionProperty);
    document.querySelector('#suite-trigger').click();
    document.querySelector('#suite-trigger').click();
    document.querySelector('#suite-trigger').click();
    return transitions;
  });
  expect(properties).toContain('width');
  expect(properties).toContain('height');
  await expect(page.locator('#suite-shell')).toHaveCSS('width', '55px');
  await expect(page.locator('#suite-menu')).toHaveJSProperty('inert', true);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.locator('#suite-trigger').click();
  await expect(page.locator('#suite-shell')).toHaveCSS('transition-duration', '0s');
  await expect(page.locator('#suite-shell')).toHaveCSS('width', '224px');
  await expect(page.locator('.app-header .title-block')).toHaveCSS('opacity', '0');
  await page.keyboard.press('Escape');
  await expect(page.locator('#suite-menu')).toBeHidden();
});

test('short drawer can scroll to its last entry without moving the logo', async ({ page }) => {
  await page.setViewportSize({ width: 560, height: 240 });
  await setup(page);
  await page.locator('#suite-trigger').click();
  await expect(page.locator('#suite-shell')).toHaveCSS('height', '216px');
  await expect(page.locator('#suite-shell')).toHaveCSS('width', '224px');
  const before = await page.locator('#suite-trigger').boundingBox();
  await page.locator('#suite-menu').evaluate(el => el.scrollTop = el.scrollHeight);
  const last = await page.locator('.suite-item.unavailable').last().boundingBox();
  expect(last.y + last.height).toBeLessThanOrEqual(228);
  expect(await page.locator('#suite-trigger').boundingBox()).toEqual(before);
});

test('click-away cannot activate the create button behind the drawer', async ({ page }) => {
  await setup(page);
  const button = await page.locator('#create-button').boundingBox();
  await page.locator('#suite-trigger').click();
  await expect(page.locator('#suite-shell')).toHaveCSS('width', '224px');
  await page.mouse.click(button.x + button.width - 15, button.y + button.height / 2);
  await expect(page.locator('#suite-menu')).toBeHidden();
  expect(await page.evaluate(() => window.calls.some(call => call.command === 'create_project'))).toBe(false);
});
