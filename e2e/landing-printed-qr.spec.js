import { test, expect } from '@playwright/test';

// The printed QR now carries `https://shirleyshow.site/#/ticket/<door secret>`.
// A phone camera opens that URL, so a buyer who scans their own ticket lands on
// the public site. Before this spec, that path fetched `/api/public/ticket/<secret>`,
// got a 404 (the route indexes by `scan_token`) and showed nothing at all -- a
// dead end introduced by the very change that made the QR readable.
test.describe('QR impreso escaneado por el comprador', () => {
  const SECRET = '648ce69829a9d86e0e8ffda75b0f279d';

  test('el secreto de puerta no se envía a la API pública', async ({ page }) => {
    const llamadas = [];
    page.on('request', (r) => {
      if (r.url().includes('/api/public/ticket/')) llamadas.push(r.url());
    });

    await page.goto(`/#/ticket/${SECRET}`);
    await page.waitForTimeout(800);

    // The door secret must never travel as a URL to that route: it is what the
    // door compares, and the route answers 404 for it anyway.
    expect(llamadas.filter((u) => u.includes(SECRET))).toHaveLength(0);
  });

  test('la página no queda en blanco ni rota', async ({ page }) => {
    const errores = [];
    page.on('pageerror', (e) => errores.push(String(e)));

    await page.goto(`/#/ticket/${SECRET}`);
    await page.waitForTimeout(600);

    await expect(page.locator('.pres__hero')).toBeVisible();
    expect(errores).toEqual([]);
  });

  test('el enlace del comprador sí carga su entrada', async ({ page }) => {
    // Control: the real buyer link must keep working, so the guard above cannot
    // be "fixed" by refusing every hash.
    await page.route('**/api/public/ticket/st_*', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          nombre_evento: 'Shirley Show',
          fecha_hora: '2027-01-10T23:00:00Z',
          lugar: 'Lima',
          nombre_comprador: 'Mayela Salinas',
          tipo_entrada: 'General',
          precio: 20,
          estado_pago: 'Pagado',
          estado_checkin: 'No ingresó',
        }),
      });
    });

    const token = `st_${'a'.repeat(64)}`;
    await page.goto(`/#/ticket/${token}`);
    await page.waitForTimeout(800);

    await expect(page.locator('#entrada')).toBeVisible();
    await expect(page.locator('#entrada')).toContainText('Mayela Salinas');
  });
});
