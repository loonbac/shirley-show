import { test, expect } from '@playwright/test';

// The placeholder tiles in the gallery are meant to show their title, and each
// tile opens the full-screen viewer. Both behaviours exist in the original
// design and were missing from the first rebuild: the tiles were empty boxes
// that looked clickable and did nothing.

test('los placeholders de la galería muestran su título', async ({ page }) => {
  await page.goto('/');
  const tiles = page.locator('.pres__item .pres__ph-txt');
  await expect(tiles).toHaveCount(6);

  for (let i = 0; i < 6; i++) {
    const el = tiles.nth(i);
    await expect(el).toBeVisible();
    const box = await el.boundingBox();
    expect(box.width).toBeGreaterThan(0);
    expect(box.height).toBeGreaterThan(0);
    expect((await el.textContent()).trim().length).toBeGreaterThan(0);
  }
});

test('una pieza de la galería abre el visor a pantalla completa', async ({ page }) => {
  await page.goto('/');
  const visor = page.locator('#lb');
  await expect(visor).toBeHidden();

  await page.locator('.pres__item').first().click();

  await expect(visor).toBeVisible();
  const pie = page.locator('#lb-pie');
  await expect(pie).toHaveText('Shirley en escena');

  // Escape closes it, like the original.
  await page.keyboard.press('Escape');
  await expect(visor).toBeHidden();
});

test('la galería responde al teclado', async ({ page }) => {
  await page.goto('/');
  const first = page.locator('.pres__item').first();
  await expect(first).toHaveAttribute('role', 'button');
  await expect(first).toHaveAttribute('tabindex', '0');

  await first.focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('#lb')).toBeVisible();
});
