// The buyer receives a WhatsApp link shaped like `/#/ticket/st_<64 hex>`.
// That hash never reaches the server, so the landing itself must fetch the
// ticket and show the buyer their proof of purchase.
//
// The card deliberately carries no QR and no secret: `codigo_qr` is what the
// door scans, and a QR rendered on the public site would let anyone who
// photographs the buyer's screen walk in. The buyer only needs to prove the
// purchase exists; the door proves it is valid.
const { test, expect } = require('@playwright/test');

const TOKEN = 'st_' + 'a'.repeat(64);

function ticketPayload(overrides = {}) {
  return {
    nombre_evento: 'Shirley Show y sus amigos sesión 3',
    fecha_hora: '2027-01-10T23:00:00Z',
    lugar: 'Auditorio',
    nombre_comprador: 'Elizabeth Sallo Roman',
    tipo_entrada: 'General',
    precio: 'S/ 20',
    estado_pago: 'Pagado',
    estado_checkin: 'No ingresó',
    ...overrides,
  };
}

test.beforeEach(async ({ page }) => {
  await page.route(`**/api/public/ticket/${TOKEN}`, async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(ticketPayload()) });
  });
});

test('el comprador ve su entrada al abrir el enlace de WhatsApp', async ({ page }) => {
  await page.goto(`/#/ticket/${TOKEN}`);

  const tarjeta = page.locator('.pres__ticket');
  await expect(tarjeta).toBeVisible();
  await expect(tarjeta).toContainText('Elizabeth Sallo Roman');
  await expect(tarjeta).toContainText('Shirley Show y sus amigos sesión 3');
  await expect(tarjeta).toContainText('Pagado');
});

test('la entrada no muestra ningún QR ni el secreto de la puerta', async ({ page }) => {
  const pedidos = [];
  page.on('request', (r) => pedidos.push(r.url()));

  await page.goto(`/#/ticket/${TOKEN}`);
  await expect(page.locator('.pres__ticket')).toBeVisible();

  // No QR, in any form: no svg, no canvas, no image.
  expect(await page.locator('.pres__ticket svg, .pres__ticket canvas, .pres__ticket img').count()).toBe(0);

  // And the rendered card never carries the scan secret the door reads.
  const texto = await page.locator('.pres__ticket').innerText();
  expect(texto).not.toMatch(/[0-9a-fA-F]{32}/);
  const html = await page.locator('.pres__ticket').innerHTML();
  expect(html).not.toContain('<svg');
  expect(html).not.toContain('<canvas');
});

test('la entrada sigue en el dispositivo en visitas posteriores', async ({ page }) => {
  await page.goto(`/#/ticket/${TOKEN}`);
  await expect(page.locator('.pres__ticket')).toBeVisible();

  // The buyer closes WhatsApp, comes back to the plain site later, and must not
  // have to hunt for the message again.
  await page.goto('/');
  const tarjeta = page.locator('.pres__ticket');
  await expect(tarjeta).toBeVisible();
  await expect(tarjeta).toContainText('Elizabeth Sallo Roman');
});

test('un enlace inválido no rompe la página pública', async ({ page }) => {
  await page.route(`**/api/public/ticket/${TOKEN}`, async (route) => {
    await route.fulfill({ status: 404, contentType: 'application/json', body: JSON.stringify({ error: 'ticket_not_found' }) });
  });

  await page.goto(`/#/ticket/${TOKEN}`);

  // The landing still presents the artist; the card simply reports that the
  // ticket could not be read. A buyer with a wrong link must not face a blank page.
  await expect(page.locator('.pres__hero')).toBeVisible();
  await expect(page.locator('.pres__ticket')).toBeHidden();
});

test('la página pública sin enlace no muestra ninguna tarjeta', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.pres__hero')).toBeVisible();
  await expect(page.locator('.pres__ticket')).toBeHidden();
});
