import { test, expect } from '@playwright/test';

// The buyer is never sent a link. What they receive is the photo of their
// ticket, and the QR inside it encodes `https://shirleyshow.site/#/ticket/<door
// secret>`. So scanning it with a phone camera is the *only* way most buyers
// ever reach this page, and it has to show them their ticket.
//
// An earlier version refused the door secret here -- it only fetched for the
// `st_...` link token -- so a buyer who scanned their own ticket saw the landing
// page and nothing else. The owner reported exactly that after buying a ticket
// and scanning it.

const DOOR_SECRET = '648ce69829a9d86e0e8ffda75b0f279d';

// A qr_svg shaped the way the `qrcode` crate emits it: one path of `M<x> <y>h<w>v<h>`
// runs, a 20px quiet zone and 5px modules, wrapped in a 225x225 viewBox. The size
// and the finder squares matter: the landing parses those runs to redraw the QR
// with the brand's ink, so a fixture without them would pass while testing nothing.
function qrSvgFixture(size = 37) {
  const paso = 5, quiet = 20;
  const runs = [];
  const modulo = (r, c) => `M${quiet + c * paso} ${quiet + r * paso}h${paso}v${paso}`;

  // Finder patterns: the three 7x7 corners, solid as the real ones are.
  for (const [fy, fx] of [[0, 0], [0, size - 7], [size - 7, 0]]) {
    runs.push(`M${quiet + fx * paso} ${quiet + fy * paso}h${7 * paso}v${7 * paso}`);
  }
  // A scattering of data modules so the matrix is not only corners.
  for (let r = 8; r < size; r++) {
    for (let c = 0; c < size; c++) {
      if ((r * 31 + c * 17) % 5 === 0) runs.push(modulo(r, c));
    }
  }

  const dim = quiet * 2 + size * paso;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${dim}" height="${dim}" viewBox="0 0 ${dim} ${dim}"><rect width="${dim}" height="${dim}" fill="#ffffff"/><path d="${runs.join('')}" fill="#000000"/></svg>`;
}

function ticketBody(overrides = {}) {
  return JSON.stringify({
    nombre_evento: 'Shirley Show y sus Amigos Sesion 3',
    fecha_hora: '2027-01-10T23:00:00Z',
    lugar: 'Lima',
    nombre_comprador: 'Mayela Salinas',
    tipo_entrada: 'General',
    precio: 20,
    estado_pago: 'Pagado',
    estado_checkin: 'No ingresó',
    qr_svg: qrSvgFixture(),
    ...overrides,
  });
}

test.describe('QR impreso escaneado por el comprador', () => {
  test('el secreto de puerta del QR abre la entrada del comprador', async ({ page }) => {
    const pedidas = [];
    await page.route('**/api/public/ticket/**', async (route) => {
      pedidas.push(route.request().url());
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: ticketBody(),
      });
    });

    await page.goto(`/#/ticket/${DOOR_SECRET}`);

    // The card the buyer paid for must appear: this is the whole point of
    // printing the link inside the QR.
    await expect(page.locator('#entrada')).toBeVisible();
    await expect(page.locator('#entrada')).toContainText('Mayela Salinas');
    await expect(page.locator('#entrada')).toContainText('Shirley Show y sus Amigos Sesion 3');

    // And it must have been fetched by the secret the QR carries, not by some
    // other token the page invented.
    expect(pedidas.some((u) => u.includes(DOOR_SECRET))).toBe(true);
  });

  test('la entrada escaneada queda guardada en el dispositivo', async ({ page }) => {
    await page.route('**/api/public/ticket/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: ticketBody() }),
    );

    await page.goto(`/#/ticket/${DOOR_SECRET}`);
    await expect(page.locator('#entrada')).toContainText('Mayela Salinas');

    // Returning to the plain site must still show the entrance: the buyer will
    // close the tab and come back at the door.
    await page.goto('/');
    await expect(page.locator('#entrada')).toBeVisible();
    await expect(page.locator('#entrada')).toContainText('Mayela Salinas');
  });

  test('la tarjeta dibuja el QR que la puerta escanea', async ({ page }) => {
    // The QR is the credential. A card that proves the purchase but shows no QR
    // leaves a buyer with nothing to present if they left the photo at home.
    await page.route('**/api/public/ticket/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: ticketBody() }),
    );

    await page.goto(`/#/ticket/${DOOR_SECRET}`);
    await expect(page.locator('#entrada')).toBeVisible();

    await expect(page.locator('#entrada .pres__ticket-qr canvas, #entrada .pres__ticket-qr svg'))
      .toHaveCount(1);
  });

  test('la tarjeta dibuja el QR con el diseño de la foto, no con la rejilla plana', async ({ page }) => {
    // The buyer keeps a printed photo whose QR is dotted with the brand gradient.
    // A flat black grid here made the page and the photo look like two different
    // tickets, which is exactly what the owner reported.
    await page.route('**/api/public/ticket/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: ticketBody() }),
    );

    await page.goto(`/#/ticket/${DOOR_SECRET}`);
    await expect(page.locator('#entrada')).toBeVisible();

    // The brand path is a canvas; the flat server SVG is the fallback.
    const canvas = page.locator('#entrada .pres__ticket-qr canvas');
    await expect(canvas).toHaveCount(1);

    // And it must be painted in the brand gradient, not left blank.
    const tieneTinta = await canvas.evaluate((el) => {
      const ctx = el.getContext('2d');
      const { data } = ctx.getImageData(0, 0, el.width, el.height);
      let pintados = 0;
      for (let i = 0; i < data.length; i += 4) {
        // Anything that is neither white nor fully transparent counts as ink.
        if (data[i + 3] > 0 && (data[i] < 240 || data[i + 1] < 240 || data[i + 2] < 240)) pintados++;
      }
      return pintados;
    });
    expect(tieneTinta).toBeGreaterThan(1000);
  });

  test('el QR queda centrado en la tarjeta, también con el CSS antiguo en caché', async ({ page }) => {
    // Cloudflare keeps .css for hours, so a buyer can run this page with the old
    // stylesheet, where `.pres__ticket-qr` does not exist. The layout therefore
    // travels inside the HTML, which is served no-cache. Measured, not assumed:
    // the QR's centre must sit on the card's centre.
    await page.route('**/api/public/ticket/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: ticketBody() }),
    );
    // Reproduce the stale cache: strip every rule this page needs from the
    // stylesheet, keeping only what the HTML itself carries.
    await page.route('**/landing.css*', async (route) => {
      const res = await route.fetch();
      const css = (await res.text()).replace(/\.pres__ticket-qr[^{]*\{[^}]*\}/g, '');
      await route.fulfill({ status: 200, contentType: 'text/css', body: css });
    });

    await page.goto(`/#/ticket/${DOOR_SECRET}`);
    await page.setViewportSize({ width: 393, height: 852 });
    await expect(page.locator('#entrada')).toBeVisible();

    const caja = page.locator('#entrada .pres__ticket-qr');
    await expect(caja).toHaveCount(1);

    const medidas = await caja.evaluate((el) => {
      const r = el.getBoundingClientRect();
      const carta = el.closest('.pres__ticket').getBoundingClientRect();
      const hijo = el.firstElementChild.getBoundingClientRect();
      return {
        centroCaja: r.x + r.width / 2,
        centroCarta: carta.x + carta.width / 2,
        centroHijo: hijo.x + hijo.width / 2,
        anchoCaja: Math.round(r.width),
        anchoVentana: document.documentElement.clientWidth,
      };
    });

    // Within a pixel of the card's centre, and the QR inside it too.
    expect(Math.abs(medidas.centroCaja - medidas.centroCarta)).toBeLessThanOrEqual(1);
    expect(Math.abs(medidas.centroHijo - medidas.centroCaja)).toBeLessThanOrEqual(1);
    // And it must not overflow the phone.
    expect(medidas.anchoCaja).toBeLessThanOrEqual(medidas.anchoVentana);
  });

  test('la tarjeta no promete que basta con dar el nombre', async ({ page }) => {
    // `door_allows_path` in src/config.rs does not expose the ticket list to the
    // door host: the doorman can only scan a QR. The old footer said the
    // opposite, which stages a scene at the door. Assert the absence of the
    // promise, not the presence of its replacement.
    await page.route('**/api/public/ticket/**', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: ticketBody() }),
    );

    await page.goto(`/#/ticket/${DOOR_SECRET}`);
    await expect(page.locator('#entrada')).toBeVisible();

    const pie = await page.locator('#entrada .pres__ticket-pie').innerText();
    expect(pie.toLowerCase()).not.toContain('basta con dar tu nombre');
    expect(pie.toLowerCase()).toContain('qr');
  });

  test('el enlace del comprador sigue funcionando', async ({ page }) => {
    // Control: the `st_...` token must keep working, so the fix above cannot be
    // "make every hash fetch" by dropping the link token.
    await page.route('**/api/public/ticket/st_*', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: ticketBody() }),
    );

    await page.goto(`/#/ticket/st_${'a'.repeat(64)}`);

    await expect(page.locator('#entrada')).toBeVisible();
    await expect(page.locator('#entrada')).toContainText('Mayela Salinas');
  });

  test('un secreto que no existe no rompe la página', async ({ page }) => {
    // Negative control: a bad scan must leave the landing usable, with no card
    // invented out of nothing.
    const errores = [];
    page.on('pageerror', (e) => errores.push(String(e)));

    await page.route('**/api/public/ticket/**', (route) =>
      route.fulfill({
        status: 404,
        contentType: 'application/json',
        body: JSON.stringify({ error: { code: 'ticket_not_found', message: 'Entrada no encontrada' } }),
      }),
    );

    await page.goto('/#/ticket/00000000000000000000000000000000');
    await page.waitForTimeout(600);

    await expect(page.locator('.pres__hero')).toBeVisible();
    await expect(page.locator('#entrada')).toBeHidden();
    expect(errores).toEqual([]);
  });
});
