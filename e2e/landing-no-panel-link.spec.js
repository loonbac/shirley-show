// The public landing is anonymous traffic. The panel lives behind Cloudflare
// Access on its own host, and `/api/agent/chat` spends paid provider tokens on
// every anonymous call. So the landing must not name, link or forward to the
// panel: not by a string, not by an anchor, not by a hash redirect.
const { test, expect } = require('@playwright/test');

for (const [w, h, name] of [[1440, 900, 'escritorio'], [390, 844, 'movil']]) {
  test(`la landing en ${name} no expone el panel y sigue completa`, async ({ page }) => {
    const errores = [];
    page.on('pageerror', (e) => errores.push(e.message));

    await page.setViewportSize({ width: w, height: h });
    await page.goto('/', { waitUntil: 'networkidle' });

    // The page itself must render as before.
    await expect(page.locator('.pres__hero')).toHaveCount(1);
    await expect(page.locator('.pres__item')).toHaveCount(6);

    // The six placeholder titles stayed visible (regression from PR #59).
    const titulos = await page
      .locator('.pres__ph-txt')
      .evaluateAll((els) => els.filter((e) => e.getBoundingClientRect().width > 0).length);
    expect(titulos).toBe(6);

    // No path to the panel, by any means.
    expect(await page.locator('a[href*="panel"]').count()).toBe(0);
    expect(await page.content()).not.toContain('panel');

    expect(errores).toEqual([]);
  });
}
