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
        suggestedProjectParent: 'F:\\UnityTest', blockers: [],
        recipe: { editorVersion: '6000.3.21f1', creatorSdkVersion: '4.0.14', urpVersion: '17.3.0', inputSystemVersion: '1.20.0' },
        editors: [{ exactRecipe: true, androidPlayer: true, androidSdk: true, androidNdk: true, openJdk: true, windowsStandalone: true, urpTemplate: 'template.tgz' }],
      };
      if (command === 'create_project') {
        if (window.options.pending) await new Promise(resolve => window.finishCreate = resolve);
        if (window.options.failCreate) throw 'Project already exists. No files were changed.';
        return { success: true, projectPath: `${args.request.parentDirectory}\\${args.request.projectName}`, message: 'Project validated.' };
      }
      if (command === 'open_project' && window.options.failOpen) throw 'Unity could not be opened.';
    } } };
  }, options);
  await page.goto(pathToFileURL(path.resolve('src/index.html')).href);
  await expect(page.locator('#create-button')).toBeEnabled();
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
