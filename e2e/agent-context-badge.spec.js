import { test, expect } from '@playwright/test';

// The mode badge in the assistant header must show its full text.
// It used to be a single line capped at 220px with an ellipsis, so the
// catalogue label read "Modo catálogo (creará un nuevo sh..." -- a truncated
// sentence that tells the user nothing.
test('el badge de modo del asistente no corta el texto', async ({ page }) => {
  await page.goto('/panel');

  // Ctrl+K opens the assistant; the floating trigger only exists once a
  // presentation exists, and this test intentionally starts from an empty
  // catalogue to reach the catalogue-mode label.
  await page.keyboard.press('Control+k');
  const badge = page.locator('.agent-chat-context-badge');
  await expect(badge).toBeVisible();

  const metrics = await badge.evaluate((el) => ({
    text: el.textContent.trim(),
    scrollWidth: el.scrollWidth,
    clientWidth: el.clientWidth,
    scrollHeight: el.scrollHeight,
    clientHeight: el.clientHeight,
  }));

  // The label must be the whole sentence, not a prefix.
  expect(metrics.text).toBe('Modo catálogo (creará un nuevo show)');

  // No horizontal clipping and no vertical clipping: the badge grows to fit.
  expect(metrics.scrollWidth).toBeLessThanOrEqual(metrics.clientWidth + 1);
  expect(metrics.scrollHeight).toBeLessThanOrEqual(metrics.clientHeight + 1);
});
