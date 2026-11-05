import { test, expect } from '@playwright/test';

// After a separation the track is the deliverable, so the confirmation card
// must let the user hear it and download it without hunting for it in the
// presentation. This mounts the real component through the page's own module
// graph, then measures the rendered result.
test('la tarjeta de pista separada ofrece descarga y reproducción', async ({ page }) => {
  await page.goto('/panel');
  await page.keyboard.press('Control+k');
  await expect(page.locator('.agent-chat-panel')).toBeVisible();

  // Render the real component with a successful add_track_from_staged action.
  await page.evaluate(async () => {
    const mod = await import('/assets/js/components/molecules/agent-tool-card.js');
    const card = mod.createAgentToolCard({
      action: {
        tool: 'add_track_from_staged',
        summary: 'Pista «Chiquitita (Coros)» añadida a la presentación',
        status: 'success',
        presentation_id: 'pres-1',
        details: { item_id: 'item-42', title: 'Chiquitita (Coros)' },
      },
    });
    card.el.id = 'probe-card';
    document.querySelector('.agent-chat-panel').appendChild(card.el);
  });

  const card = page.locator('#probe-card');
  await expect(card).toBeVisible();

  // Download link points at the existing audio endpoint with the download flag.
  const download = page.locator('#probe-card .agent-tool-card-download');
  await expect(download).toBeVisible();
  const href = await download.getAttribute('href');
  expect(href).toBe('/api/tracks/item-42/audio?download=1');
  await expect(download).toHaveAttribute('download', '');

  // The download control must be a real tap target on a phone.
  const linkBox = await download.boundingBox();
  expect(linkBox.height).toBeGreaterThanOrEqual(44);

  // Playback is offered in place, with native controls.
  const audio = page.locator('#probe-card .agent-tool-card-audio');
  await expect(audio).toHaveAttribute('controls', '');
  await expect(audio).toHaveAttribute('src', '/api/tracks/item-42/audio');
  const audioBox = await audio.boundingBox();
  expect(audioBox.height).toBeGreaterThanOrEqual(40);
  expect(audioBox.width).toBeGreaterThan(0);
});
