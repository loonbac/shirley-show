// The buyer reaches this page two ways: the `st_<64 hex>` link sent over
// WhatsApp, and the QR printed inside the photo of their ticket, which carries
// `#/ticket/<door secret>`. Both resolve to the same card, and that card shows
// the QR because it is the credential the door scans.
//
// The card does not repeat the price or the payment state. A paid ticket's QR is
// the proof and the buyer knows what they paid; unpaid ones still get the
// warning underneath, which is where that fact actually matters.
//
// What must never appear is the door secret as *plain text*: the QR encodes it
// as modules, but a copyable hex string sitting in the markup is a leak.
const { test, expect } = require('@playwright/test');

const TOKEN = 'st_' + 'a'.repeat(64);

// Shaped like the `qrcode` crate output: one path of `M<x> <y>h<w>v<h>` runs.
// The landing parses those runs to redraw the code with the brand's ink.
function qrSvgFixture(size = 45) {
  const paso = 5, quiet = 20;
  const runs = [];
  for (const [fy, fx] of [[0, 0], [0, size - 7], [size - 7, 0]]) {
    runs.push(`M${quiet + fx * paso} ${quiet + fy * paso}h${7 * paso}v${7 * paso}`);
  }
  for (let r = 8; r < size; r++) {
    for (let c = 0; c < size; c++) {
      if ((r * 31 + c * 17) % 5 === 0) {
        runs.push(`M${quiet + c * paso} ${quiet + r * paso}h${paso}v${paso}`);
      }
    }
  }
  const dim = quiet * 2 + size * paso;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${dim}" height="${dim}" viewBox="0 0 ${dim} ${dim}"><rect width="${dim}" height="${dim}" fill="#ffffff"/><path d="${runs.join('')}" fill="#000000"/></svg>`;
}

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
    qr_svg: qrSvgFixture(),
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
  await expect(tarjeta).toContainText('Tipo');
  // Neither the price nor the payment state is repeated on a paid ticket.
  const texto = await tarjeta.innerText();
  expect(texto).not.toMatch(/Pagado/);
  expect(texto).not.toMatch(/Precio/i);
});

test('la tarjeta muestra el QR pero nunca el secreto como texto', async ({ page }) => {
  await page.goto(`/#/ticket/${TOKEN}`);
  await expect(page.locator('.pres__ticket')).toBeVisible();

  // The QR *is* the credential the door scans, so it must be there.
  await expect(page.locator('.pres__ticket .pres__ticket-qr canvas')).toHaveCount(1);

  // What must never be there is the secret as a copyable hex string: the code
  // encodes it as modules, the markup must not spell it out.
  const texto = await page.locator('.pres__ticket').innerText();
  expect(texto).not.toMatch(/[0-9a-fA-F]{32}/);
  const html = await page.locator('.pres__ticket').innerHTML();
  expect(html).not.toMatch(/[0-9a-fA-F]{32}/);
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
