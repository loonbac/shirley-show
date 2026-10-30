import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 45000,
  expect: {
    timeout: 10000,
  },
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [['list']],
  use: {
    baseURL: 'http://127.0.0.1:4455',
    browserName: 'chromium',
    permissions: ['camera'],
    launchOptions: {
      executablePath: '/run/current-system/sw/bin/chromium',
      headless: true,
      args: [
        '--no-sandbox',
        '--disable-setuid-sandbox',
        '--use-fake-ui-for-media-stream',
        '--use-fake-device-for-media-stream',
      ],
    },
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  webServer: {
    command: 'rm -rf /tmp/shirley-playwright-data && mkdir -p /tmp/shirley-playwright-data && SHIRLEY_BIND="127.0.0.1:4455" SHIRLEY_DATA_DIR="/tmp/shirley-playwright-data" SHIRLEY_FRONTEND_DIR="./frontend" /var/cache/rust-target/home-loonbac-Proyectos-shirley-show/debug/shirley-show',
    url: 'http://127.0.0.1:4455/api/presentations',
    reuseExistingServer: false,
    timeout: 15000,
  },
});
