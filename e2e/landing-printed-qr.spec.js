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
    qr_svg: '<svg xmlns="http://www.w3.org/2000/svg"></svg>',
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

    await expect(page.locator('#entrada .pres__ticket-qr svg')).toHaveCount(1);
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
