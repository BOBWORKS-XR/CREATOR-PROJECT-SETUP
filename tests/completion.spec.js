const { test, expect } = require('@playwright/test');
const { pathToFileURL } = require('node:url');
const path = require('node:path');

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
        platform: 'windows', ready: true, hubInstalled: true, unityCliInstalled: false,
        hubVersion: window.options.legacyHub ? '3.14.4' : '3.21.1', hubAutoRegistration: !window.options.legacyHub,
        suggestedProjectParent: 'F:\\UnityTest', blockers: [],
        recipe: { editorVersion: '6000.3.21f1', creatorSdkVersion: '4.0.14', urpVersion: '17.3.0', inputSystemVersion: '1.20.0' },
        editors: [{ exactRecipe: true, androidPlayer: true, androidSdk: true, androidNdk: true, openJdk: true, windowsStandalone: true, urpTemplate: 'template.tgz' }],
      };
      if (command === 'create_project') {
        if (window.options.pending) await new Promise(resolve => window.finishCreate = resolve);
        if (window.options.failCreate) throw 'Project already exists. No files were changed.';
        return { success: true, projectPath: `${args.request.parentDirectory}\\${args.request.projectName}`, message: 'Project validated.', hub: window.options.legacyHub ? { registered: false, requiresHubUpdate: true, message: 'Unity Hub 3.14.4 uses the old registry. Update to 3.21.1 or use Add > Add project from disk.' } : { registered: !window.options.failHub, message: window.options.failHub ? 'Download failed. Your project is ready to open.' : "Registered in Unity Hub's project database. Check Hub's Projects list." } };
      }
      if (command === 'register_project') return { registered: true, message: "Registered in Unity Hub's project database. Check Hub's Projects list." };
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
  await page.goto(pathToFileURL(path.resolve('src/index.html')).href);
  await expect(page.locator('#create-button')).toBeEnabled();
}

test('Hub failure preserves completed project and retry does not recreate it', async ({ page }) => {
  await setup(page, { failHub: true });
  await page.locator('#create-button').click();
  await expect(page.locator('#hub-result')).toContainText('Download failed');
  await expect(page.locator('#open-project-button')).toBeEnabled();
  await page.locator('#retry-hub-button').click();
  await expect(page.locator('#hub-result')).toContainText("Registered in Unity Hub's project database.");
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
  await expect(page.locator('#hub-result')).toHaveClass('hub-result');
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
    expect(await page.locator('.brand img').evaluate(img => img.naturalWidth > 0)).toBe(true);
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
});

test('creating another project explicitly unlocks a new name', async ({ page }) => {
  await setup(page);
  await page.locator('#create-button').click();
  await page.locator('#create-another-button').click();
  await expect(page.locator('#project-name')).toBeEmpty();
  await expect(page.locator('#project-name')).toBeFocused();
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
