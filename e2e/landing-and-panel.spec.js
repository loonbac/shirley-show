import { test, expect } from '@playwright/test';

// The site root is now the public landing. The panel moved to /panel.
// Tickets were already sold and their links shared, so the legacy URLs MUST
// keep landing on something useful. These tests treat those old links as the
// contract they are.

test('la raíz sirve la landing pública, no el panel', async ({ page }) => {
  await page.goto('/');

  // Landing markers.
  await expect(page.locator('.pres-hero')).toBeVisible();
  await expect(page.getByText('Música en vivo para tus eventos')).toBeVisible();

  // The panel UI must not be what loads at the root.
  await expect(page.locator('.agent-chat-panel')).toHaveCount(0);
  await expect(page.locator('.app-topbar')).toHaveCount(0);
});

test('el panel sigue accesible en /panel', async ({ page }) => {
  await page.goto('/panel');
  await expect(page.locator('.app-topbar')).toBeVisible();
});

test('un enlace viejo de entrada (#/ticket/...) sigue llegando al panel', async ({ page }) => {
  // This is the exact shape already shared with buyers over WhatsApp.
  await page.goto('/#/ticket/st_ab12cd34ef56ab78cd90ef12ab34cd56');

  // The landing forwards to the panel preserving the hash, and the public
  // ticket view resolves it. The token here is invented, so the view reports
  // that it does not exist -- which proves it was reached, not lost.
  await expect(page).toHaveURL(/\/panel#\/ticket\/st_ab12cd34ef56ab78cd90ef12ab34cd56$/);
  await expect(page.getByText('Entrada no disponible')).toBeVisible();
});

test('un enlace viejo de evento (#/eventos/...) sigue llegando al panel', async ({ page }) => {
  await page.goto('/#/eventos/abc-123');
  await expect(page).toHaveURL(/\/panel#\/eventos\/abc-123$/);
});

test('la raíz sin hash no redirige: se queda en la landing', async ({ page }) => {
  await page.goto('/');
  await page.waitForTimeout(400);
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('.pres-hero')).toBeVisible();
});
