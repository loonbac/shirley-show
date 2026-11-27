import { test, expect } from '@playwright/test';

// S4: the chat is global. Moving between screens while a separation is running
// must not abort the fetch, drop the transcript or switch the chosen repertoire.
test('el chat sigue separando al navegar y mantiene el repertorio elegido', async ({ page }) => {
  const chatRequests = [];
  const failedChatRequests = [];
  page.on('requestfailed', (request) => {
    if (request.url().includes('/api/agent/chat')) {
      failedChatRequests.push(request.failure()?.errorText || 'unknown');
    }
  });

  await page.route('**/api/agent/chat', async (route) => {
    chatRequests.push(route);
  });

  await page.goto('/panel');
  await page.keyboard.press('Control+k');
  await expect(page.locator('.agent-chat-panel')).toBeVisible();

  await page.locator('.agent-composer-textarea').fill('Separa los coros y guárdalos en Peña del Viernes');
  await page.locator('.agent-composer-textarea').press('Enter');
  await expect.poll(() => chatRequests.length).toBe(1);

  const firstBody = JSON.parse(chatRequests[0].request().postData() || '{}');
  expect(firstBody.presentation_id).toBe(null, 'sin repertorio abierto, presentation_id sigue siendo opcional');

  await page.evaluate(async () => {
    const mod = await import('/assets/js/core/state/agent-refresh.js');
    window.__agentRefreshEvents = [];
    window.__unsubscribeAgentRefresh = mod.onAgentRefresh((event) => {
      window.__agentRefreshEvents.push(event);
    });
  });

  // El usuario se va a otra pantalla con la petición todavía en vuelo.
  await page.evaluate(() => {
    window.location.hash = '#/events';
  });
  await expect(page.locator('.agent-chat-panel')).toBeVisible();
  await expect(page.locator('.agent-typing-indicator')).toHaveCount(1);
  expect(failedChatRequests, 'navegar no aborta la separación en curso').toEqual([]);

  await chatRequests[0].fulfill({
    status: 200,
    contentType: 'application/json',
    body: JSON.stringify({
      message: 'Pista separada y guardada en Peña del Viernes',
      actions_taken: [],
      active_presentation_id: 'pres-elegido',
      active_presentation_title: 'Peña del Viernes',
      conversation_id: 'conv-e2e-navigation',
      notice: null,
    }),
  });

  await expect(page.locator('.agent-chat-context-badge')).toContainText('Peña del Viernes');
  await expect(page.locator('.agent-message-text').last()).toContainText('Pista separada y guardada');

  // Vuelta al catálogo: ni el badge ni la conversación se reinician.
  await page.evaluate(() => {
    window.location.hash = '#/presentations';
  });
  await expect(page.locator('.agent-chat-context-badge')).toContainText('Peña del Viernes');
  await expect(page.locator('.agent-message-text').last()).toContainText('Pista separada y guardada');

  const refreshEvents = await page.evaluate(() => window.__agentRefreshEvents);
  expect(
    refreshEvents.some((event) => event.scope === 'presentation' && event.presentationId === 'pres-elegido'),
    'la vista del repertorio destino se refresca aunque el usuario esté en otra ruta'
  ).toBe(true);

  await page.locator('.agent-composer-textarea').fill('Agrega otra pista al mismo show');
  await page.locator('.agent-composer-textarea').press('Enter');
  await expect.poll(() => chatRequests.length).toBe(2);

  const secondBody = JSON.parse(chatRequests[1].request().postData() || '{}');
  expect(secondBody.presentation_id).toBe('pres-elegido');
  expect(secondBody.conversation_id).toBe('conv-e2e-navigation');

  await chatRequests[1].fulfill({
    status: 200,
    contentType: 'application/json',
    body: JSON.stringify({
      message: 'Listo',
      actions_taken: [],
      active_presentation_id: 'pres-elegido',
      conversation_id: 'conv-e2e-navigation',
      notice: null,
    }),
  });

  await expect(page.locator('.agent-message-text').last()).toContainText('Listo');
  expect(failedChatRequests, 'la segunda separación tampoco debe abortarse').toEqual([]);
  await page.evaluate(async () => {
    if (typeof window.__unsubscribeAgentRefresh === 'function') {
      window.__unsubscribeAgentRefresh();
    }
  });
});
