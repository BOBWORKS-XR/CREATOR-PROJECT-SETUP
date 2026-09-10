const { defineConfig } = require('@playwright/test');

module.exports = defineConfig({
  testDir: './tests',
  use: { browserName: 'chromium', viewport: { width: 980, height: 760 } },
});
