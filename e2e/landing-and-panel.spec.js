import { test, expect } from '@playwright/test';

// The site root is now the public landing. The panel moved to /panel.
// Tickets were already sold and their links shared, so the legacy URLs MUST
// keep landing on something useful. These tests treat those old links as the
// contract they are.

test('la raíz sirve la landing pública, no el panel', async ({ page }) => {
  await page.goto('/');

  // Landing markers.
  await expect(page.locator('.pres__hero')).toBeVisible();
  await expect(page.getByText('Música en vivo para tus eventos')).toBeVisible();

  // The panel UI must not be what loads at the root.
  await expect(page.locator('.agent-chat-panel')).toHaveCount(0);
  await expect(page.locator('.app-topbar')).toHaveCount(0);
});

test('el panel sigue accesible en /panel', async ({ page }) => {
  await page.goto('/panel');
  await expect(page.locator('.app-topbar')).toBeVisible();
});

test('la landing no reenvía al panel por un hash de entrada', async ({ page }) => {
  // This exact shape was shared with buyers over WhatsApp. It used to land them
  // in the panel, which is behind Access on its own host; the landing handed
  // out the protected surface to anyone who typed a fragment.
  await page.goto('/#/ticket/st_ab12cd34ef56ab78cd90ef12ab34cd56');

  await expect(page).toHaveURL(/^[^#]*\/#\/ticket\/st_ab12cd34ef56ab78cd90ef12ab34cd56$/);
  await expect(page.locator('.pres__hero')).toBeVisible();
});

test('la landing no reenvía al panel por un hash de evento', async ({ page }) => {
  await page.goto('/#/eventos/abc-123');
  await page.waitForTimeout(400);
  await expect(page).not.toHaveURL(/\/panel/);
  await expect(page.locator('.pres__hero')).toBeVisible();
});

test('la landing pública no menciona el panel en ningún sitio', async ({ page }) => {
  // Not just no link: no string, no comment, nothing that names the protected
  // surface to an anonymous visitor.
  await page.goto('/');
  const html = await page.content();
  expect(html).not.toContain('panel');
  expect(await page.locator('a[href*="panel"]').count()).toBe(0);
});

test('la raíz sin hash no redirige: se queda en la landing', async ({ page }) => {
  await page.goto('/');
  await page.waitForTimeout(400);
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('.pres__hero')).toBeVisible();
});
