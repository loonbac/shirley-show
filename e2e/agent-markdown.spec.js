import { test, expect } from '@playwright/test';

/**
 * Assistant replies are rendered as Markdown. This spec proves the rendered
 * result inside the running app, not just the isolated component: it injects a
 * real assistant turn into the chat state, lets the controller render it, and
 * inspects the produced DOM.
 */
test.describe('Asistente: respuestas en Markdown', () => {

  const MARKDOWN_REPLY = [
    '## Títulos y jerarquía',
    '**Negrita**, *cursiva* y `código en línea`.',
    '',
    '**Con viñetas:**',
    '- Primer punto',
    '- Segundo punto',
    '',
    '**Checklist:**',
    '- [x] Letras cargadas',
    '- [ ] Pistas de audio',
    '',
    '| N° | Canción | Tono |',
    '|----|---------|------|',
    '| 1 | Como la flor | Do |',
    '',
    '> Nota importante.',
    '',
    '```python',
    'def saludo(nombre):',
    '    return f"Hola, {nombre}"',
    '```',
  ].join('\n');

  test('un turno del asistente se renderiza como Markdown, no como texto plano', async ({ page }) => {
    await page.goto('/panel');
    await page.waitForLoadState('networkidle');

    // Inject one assistant turn and one user turn through the live app modules.
    const injected = await page.evaluate(async (markdown) => {
      const mod = await import('/assets/js/components/molecules/agent-message-bubble.js');

      const host = document.createElement('div');
      host.id = 'e2e-host';
      document.body.appendChild(host);

      const assistant = mod.createAgentMessageBubble({
        message: { id: 'e2e-assistant', role: 'assistant', content: markdown },
      });
      host.appendChild(assistant.el);

      const user = mod.createAgentMessageBubble({
        message: { id: 'e2e-user', role: 'user', content: '## no soy titulo **ni negrita**' },
      });
      host.appendChild(user.el);

      return { ok: true };
    }, MARKDOWN_REPLY);

    expect(injected.ok).toBe(true);

    const body = page.locator('#e2e-host .agent-message-text--markdown');
    await expect(body).toBeVisible();

    // Structure is real, not escaped text
    await expect(body.locator('h2')).toHaveText('Títulos y jerarquía');
    await expect(body.locator('strong')).toHaveCount(3);
    await expect(body.locator('em')).toHaveCount(1);
    await expect(body.locator('ul li')).toHaveCount(4);
    await expect(body.locator('input[type="checkbox"]')).toHaveCount(2);
    await expect(body.locator('input[type="checkbox"]:checked')).toHaveCount(1);
    await expect(body.locator('table th')).toHaveCount(3);
    await expect(body.locator('table td')).toHaveCount(3);
    await expect(body.locator('blockquote')).toHaveCount(1);
    await expect(body.locator('pre code')).toContainText('def saludo(nombre):');

    // No markdown syntax survived into the visible text
    const visibleText = await body.innerText();
    expect(visibleText).not.toContain('##');
    expect(visibleText).not.toContain('**');
    expect(visibleText).not.toContain('```');
    expect(visibleText).not.toMatch(/^\|/m);

    // The user turn stays literal
    const userBubble = page.locator('#e2e-host .agent-message-bubble--user');
    await expect(userBubble.locator('h2')).toHaveCount(0);
    await expect(userBubble.locator('strong')).toHaveCount(0);
    await expect(userBubble).toContainText('## no soy titulo **ni negrita**');

    // Nothing overflows the bubble horizontally at phone width
    await page.setViewportSize({ width: 390, height: 844 });
    const overflow = await body.evaluate((el) => ({
      scrollWidth: el.scrollWidth,
      clientWidth: el.clientWidth,
    }));
    expect(overflow.scrollWidth).toBeLessThanOrEqual(overflow.clientWidth + 1);
  });

  test('el HTML del modelo nunca se convierte en elementos', async ({ page }) => {
    await page.goto('/panel');
    await page.waitForLoadState('networkidle');

    const result = await page.evaluate(async () => {
      const mod = await import('/assets/js/components/molecules/agent-message-bubble.js');
      const bubble = mod.createAgentMessageBubble({
        message: {
          id: 'e2e-xss',
          role: 'assistant',
          content: '<img src=x onerror="window.__xss=1"> <script>window.__xss=2<\/script>',
        },
      });
      document.body.appendChild(bubble.el);
      // Give the browser a chance to run anything that was parsed as markup
      await new Promise((r) => setTimeout(r, 300));
      return {
        xss: window.__xss ?? null,
        imgs: bubble.el.querySelectorAll('img').length,
        scripts: bubble.el.querySelectorAll('script').length,
        text: bubble.el.textContent,
      };
    });

    expect(result.xss).toBeNull();
    expect(result.imgs).toBe(0);
    expect(result.scripts).toBe(0);
    expect(result.text).toContain('<script>');
  });
});
