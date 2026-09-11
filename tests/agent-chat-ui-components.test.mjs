import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { setupDomEnvironment } from './fake-dom.mjs';

setupDomEnvironment();

const iconModulePath = resolve(process.cwd(), 'frontend/js/components/atoms/icon.js');
const { createIcon } = await import(pathToFileURL(iconModulePath).href);

const triggerModulePath = resolve(process.cwd(), 'frontend/js/components/molecules/agent-chat-trigger.js');
const { createAgentChatTrigger } = await import(pathToFileURL(triggerModulePath).href);

const bubbleModulePath = resolve(process.cwd(), 'frontend/js/components/molecules/agent-message-bubble.js');
const { createAgentMessageBubble } = await import(pathToFileURL(bubbleModulePath).href);

const typingModulePath = resolve(process.cwd(), 'frontend/js/components/molecules/agent-typing-indicator.js');
const { createAgentTypingIndicator } = await import(pathToFileURL(typingModulePath).href);

const chipModulePath = resolve(process.cwd(), 'frontend/js/components/molecules/agent-attachment-chip.js');
const { createAgentAttachmentChip } = await import(pathToFileURL(chipModulePath).href);

const toolCardModulePath = resolve(process.cwd(), 'frontend/js/components/molecules/agent-tool-card.js');
const { createAgentToolCard } = await import(pathToFileURL(toolCardModulePath).href);

const composerModulePath = resolve(process.cwd(), 'frontend/js/components/organisms/agent-chat-composer.js');
const { createAgentChatComposer } = await import(pathToFileURL(composerModulePath).href);

const panelModulePath = resolve(process.cwd(), 'frontend/js/components/organisms/agent-chat-panel.js');
const { createAgentChatPanel } = await import(pathToFileURL(panelModulePath).href);

const shortcutsModulePath = resolve(process.cwd(), 'frontend/js/behaviors/bind-agent-chat-shortcuts.js');
const { bindAgentChatShortcuts } = await import(pathToFileURL(shortcutsModulePath).href);

const focusTrapModulePath = resolve(process.cwd(), 'frontend/js/behaviors/use-focus-trap.js');
const { useFocusTrap } = await import(pathToFileURL(focusTrapModulePath).href);

const stickToBottomModulePath = resolve(process.cwd(), 'frontend/js/behaviors/use-stick-to-bottom.js');
const { useStickToBottom } = await import(pathToFileURL(stickToBottomModulePath).href);

console.log('Running Agent Chat UI Components and Behaviors Test Harness...');

// 1. New SVG icons in icon.js
{
  const sparkles = createIcon({ name: 'sparkles' });
  assert.ok(sparkles.getAttribute('class').includes('icon'));
  assert.ok(sparkles.innerHTML.length > 0);

  const send = createIcon({ name: 'send' });
  assert.ok(send.getAttribute('class').includes('icon'));
  assert.ok(send.innerHTML.length > 0);

  const paperclip = createIcon({ name: 'paperclip' });
  assert.ok(paperclip.getAttribute('class').includes('icon'));
  assert.ok(paperclip.innerHTML.length > 0);

  const bot = createIcon({ name: 'bot' });
  assert.ok(bot.getAttribute('class').includes('icon'));
  assert.ok(bot.innerHTML.length > 0);

  console.log('✓ 1. Required SVG icons (sparkles, send, paperclip, bot) present and valid');
}

// 2. Molecule: Agent Chat Trigger (touch target >= 48px, label, shortcut)
{
  let clicked = false;
  const trigger = createAgentChatTrigger({
    onClick: () => { clicked = true; },
  });

  assert.ok(trigger.el.classList.contains('agent-chat-trigger'));
  assert.equal(trigger.el.getAttribute('type'), 'button');
  assert.ok(trigger.el.textContent.includes('Asistente Shirley'));
  assert.ok(trigger.el.textContent.includes('K')); // Keyboard shortcut hint

  trigger.el.dispatchEvent(new Event('click'));
  assert.equal(clicked, true);

  console.log('✓ 2. Agent chat trigger molecule has accessible label, shortcut hint, and handles click');
}

// 3. Molecule: Agent Message Bubble (role distinction, safe text escaping, tool cards)
{
  // User bubble
  const userBubble = createAgentMessageBubble({
    message: {
      id: 'm1',
      role: 'user',
      content: 'Hola Shirley',
      timestamp: Date.now(),
    },
  });
  assert.ok(userBubble.el.classList.contains('agent-message-bubble'));
  assert.ok(userBubble.el.classList.contains('agent-message-bubble--user'));
  assert.ok(userBubble.el.textContent.includes('Hola Shirley'));

  // Assistant bubble with potentially malicious HTML / script
  const maliciousContent = `<script>alert('xss')</script><img src=x onerror=alert(1)>Verso 1\nVerso 2`;
  const assistantBubble = createAgentMessageBubble({
    message: {
      id: 'm2',
      role: 'assistant',
      content: maliciousContent,
      actions: [
        {
          tool: 'create_presentation',
          presentation_id: 'pres-1',
          summary: 'Presentación creada',
          status: 'success',
        },
      ],
      timestamp: Date.now(),
    },
  });

  assert.ok(assistantBubble.el.classList.contains('agent-message-bubble--assistant'));
  // Safe escaping verification: script and img tags must NOT exist as element nodes!
  assert.equal(assistantBubble.el.querySelectorAll('script').length, 0, 'No active script tags allowed');
  assert.equal(assistantBubble.el.querySelectorAll('img').length, 0, 'No active img tags allowed');
  // Text content must preserve literal text and newlines
  assert.ok(assistantBubble.el.textContent.includes("<script>alert('xss')</script>"));
  assert.ok(assistantBubble.el.textContent.includes("Verso 1\nVerso 2"));
  // Tool card rendered
  assert.ok(assistantBubble.el.querySelector('.agent-tool-card') !== null);

  console.log('✓ 3. Agent message bubble renders roles cleanly, safely escapes malicious strings, and displays tool receipts');
}

// 4. Molecule: Agent Typing Indicator (golden dots + accessible live region)
{
  const indicator = createAgentTypingIndicator();
  assert.ok(indicator.el.classList.contains('agent-typing-indicator'));
  assert.equal(indicator.el.getAttribute('aria-live'), 'polite');
  assert.ok(indicator.el.textContent.includes('Shirley está pensando...'));
  assert.ok(indicator.el.querySelector('.agent-typing-dots') !== null);

  console.log('✓ 4. Agent typing indicator has aria-live="polite" and pulsing dots container');
}

// 5. Molecule: Agent Attachment Chip
{
  let removed = false;
  const chip = createAgentAttachmentChip({
    file: { name: 'audio_grabacion.mp3', size: 5.4 * 1024 * 1024 },
    onRemove: () => { removed = true; },
  });

  assert.ok(chip.el.classList.contains('agent-attachment-chip'));
  assert.ok(chip.el.textContent.includes('audio_grabacion.mp3'));
  assert.ok(chip.el.textContent.includes('5.4 MB'));

  const removeBtn = chip.el.querySelector('.agent-attachment-chip-remove');
  assert.ok(removeBtn !== null);
  assert.ok(removeBtn.getAttribute('aria-label') !== null);

  removeBtn.dispatchEvent(new Event('click'));
  assert.equal(removed, true);

  console.log('✓ 5. Agent attachment chip displays file name, formatted size, and accessible remove button');
}

// 6. Molecule: Agent Tool Card
{
  const card = createAgentToolCard({
    action: {
      tool: 'add_lyrics',
      summary: 'Letra "Estribillo" añadida al show',
      status: 'success',
    },
  });

  assert.ok(card.el.classList.contains('agent-tool-card'));
  assert.ok(card.el.textContent.includes('Letra "Estribillo" añadida al show'));
  assert.ok(card.el.querySelector('.agent-tool-card-icon') !== null);

  console.log('✓ 6. Agent tool card renders action receipt badge');
}

// 7. Organism: Agent Chat Composer (keyboard handlers, disabled state)
{
  let sentMessage = null;
  let draftChanged = null;
  let attachClicked = false;

  const composer = createAgentChatComposer({
    draft: 'Letra de canción',
    attachment: null,
    pending: false,
    onSend: (msg) => { sentMessage = msg; },
    onChangeDraft: (val) => { draftChanged = val; },
    onAttachClick: () => { attachClicked = true; },
    onRemoveAttachment: () => {},
  });

  assert.ok(composer.el.classList.contains('agent-chat-composer'));
  const textarea = composer.textareaEl;
  assert.equal(textarea.value, 'Letra de canción');

  const attachBtn = composer.attachBtnEl;
  attachBtn.dispatchEvent(new Event('click'));
  assert.equal(attachClicked, true);

  // Shift+Enter does NOT submit
  const shiftEnterEv = new Event('keydown');
  shiftEnterEv.key = 'Enter';
  shiftEnterEv.shiftKey = true;
  shiftEnterEv.preventDefault = () => {};
  textarea.dispatchEvent(shiftEnterEv);
  assert.equal(sentMessage, null, 'Shift+Enter must not submit');

  // Enter submits
  const enterEv = new Event('keydown');
  enterEv.key = 'Enter';
  enterEv.shiftKey = false;
  let defaultPrevented = false;
  enterEv.preventDefault = () => { defaultPrevented = true; };
  textarea.dispatchEvent(enterEv);
  assert.equal(defaultPrevented, true);
  assert.equal(sentMessage, 'Letra de canción');

  // When pending: controls disabled
  composer.update({ pending: true });
  assert.equal(textarea.hasAttribute('disabled'), true);
  assert.equal(attachBtn.hasAttribute('disabled'), true);
  assert.equal(composer.sendBtnEl.hasAttribute('disabled'), true);

  console.log('✓ 7. Agent chat composer handles Enter, Shift+Enter, and pending disablement');
}

// 8. Organism: Agent Chat Panel (dialog accessibility, context badge, backdrop)
{
  let closed = false;
  const panel = createAgentChatPanel({
    isOpen: true,
    activeContext: { id: 'show-1', title: 'Concierto Acústico' },
    messages: [],
    pending: false,
    draft: '',
    attachment: null,
    onClose: () => { closed = true; },
    onSend: () => {},
    onChangeDraft: () => {},
    onAttachClick: () => {},
    onRemoveAttachment: () => {},
  });

  const dialogEl = panel.dialogEl;
  assert.equal(dialogEl.getAttribute('role'), 'dialog');
  assert.equal(dialogEl.getAttribute('aria-modal'), 'true');

  // Context badge displays show title
  const badgeEl = panel.contextBadgeEl;
  assert.ok(badgeEl.textContent.includes('Concierto Acústico'));

  // Updating to catalog mode
  panel.update({ activeContext: null });
  assert.ok(badgeEl.textContent.includes('Modo catálogo'));

  // Close button
  const closeBtn = panel.closeBtnEl;
  closeBtn.dispatchEvent(new Event('click'));
  assert.equal(closed, true);

  console.log('✓ 8. Agent chat panel dialog attributes, context badge, and close action');
}

// 9. Behavior: Shortcuts and Stage Mode suppression
{
  let toggledCount = 0;
  let isStage = false;

  const cleanup = bindAgentChatShortcuts({
    onToggleChat: () => { toggledCount += 1; },
    isStageModeActive: () => isStage,
  });

  // Ctrl+K when Stage Mode is inactive -> toggles chat
  const ctrlKEv = new Event('keydown');
  ctrlKEv.key = 'k';
  ctrlKEv.ctrlKey = true;
  ctrlKEv.preventDefault = () => {};
  window.dispatchEvent(ctrlKEv);
  assert.equal(toggledCount, 1, 'Ctrl+K should toggle chat when not in Stage Mode');

  // Cmd+K (metaKey) on Mac
  const cmdKEv = new Event('keydown');
  cmdKEv.key = 'k';
  cmdKEv.metaKey = true;
  cmdKEv.preventDefault = () => {};
  window.dispatchEvent(cmdKEv);
  assert.equal(toggledCount, 2, 'Cmd+K should toggle chat when not in Stage Mode');

  // In Stage Mode: Ctrl+K is suppressed!
  isStage = true;
  window.dispatchEvent(ctrlKEv);
  assert.equal(toggledCount, 2, 'Ctrl+K must be suppressed during Stage Mode');

  cleanup();
  console.log('✓ 9. Global Ctrl+K / Cmd+K shortcut listener with Stage Mode suppression');
}

// 10. TRIANGULATE: Edge cases in UI rendering
{
  // A. Extremely long filenames without overflowing composer
  const longName = 'grabacion_ensayo_acustico_en_vivo_con_guitarra_y_arreglos_de_cuerdas_final_v2_completa_2026.mp3';
  const chipLong = createAgentAttachmentChip({
    file: { name: longName, size: 25 * 1024 * 1024 },
    onRemove: () => {},
  });
  assert.ok(chipLong.el.querySelector('.agent-attachment-chip-name').textContent === longName);
  assert.ok(chipLong.el.classList.contains('agent-attachment-chip'));

  // B. Multiline lyrics formatting preserved with line breaks
  const stanzas = 'Estrofa 1:\nBajo la luz dorada del escenario\n\nEstrofa 2:\nShirley canta su melodía';
  const lyricBubble = createAgentMessageBubble({
    message: {
      id: 'lyr-1',
      role: 'assistant',
      content: stanzas,
      timestamp: Date.now(),
    },
  });
  const textContent = lyricBubble.el.querySelector('.agent-message-text').textContent;
  assert.equal(textContent, stanzas, 'Lyrics must strictly preserve multi-line breaks');

  // C. Disabled state during pending prevents duplicate submits
  let sendCount = 0;
  const pendingComposer = createAgentChatComposer({
    draft: 'Mensaje pendiente',
    attachment: null,
    pending: true,
    onSend: () => { sendCount += 1; },
  });

  // Try Enter keydown while pending
  const enterEv = new Event('keydown');
  enterEv.key = 'Enter';
  enterEv.preventDefault = () => {};
  pendingComposer.textareaEl.dispatchEvent(enterEv);
  assert.equal(sendCount, 0, 'Enter key must NOT trigger send while pending');

  // Try clicking send button while pending
  pendingComposer.sendBtnEl.dispatchEvent(new Event('click'));
  assert.equal(sendCount, 0, 'Send button click must NOT trigger send while pending');

  // D. prefers-reduced-motion respected in useStickToBottom
  let scrollToCall = null;
  const mockContainer = {
    scrollHeight: 1000,
    scrollTop: 900,
    clientHeight: 100,
    scrollTo: (opts) => { scrollToCall = opts; },
  };

  // Mock window.matchMedia for reduced motion = true
  window.matchMedia = (query) => {
    if (query === '(prefers-reduced-motion: reduce)') {
      return { matches: true };
    }
    return { matches: false };
  };

  const stickAuto = useStickToBottom(mockContainer);
  stickAuto.scrollToBottom(true);
  assert.equal(scrollToCall.behavior, 'auto', 'Behavior must be "auto" when prefers-reduced-motion is true');

  // Mock window.matchMedia for reduced motion = false
  window.matchMedia = () => ({ matches: false });
  const stickSmooth = useStickToBottom(mockContainer);
  stickSmooth.scrollToBottom(true);
  assert.equal(scrollToCall.behavior, 'smooth', 'Behavior must be "smooth" when prefers-reduced-motion is false');

  console.log('✓ 10. Triangulation edge cases (long filenames, lyrics multiline, pending lock, reduced motion)');
}

// 11. Escape key dismissal and focus restoration
{
  let closeTriggered = false;
  const triggerBtn = document.createElement('button');
  triggerBtn.setAttribute('id', 'origin-trigger');
  document.body.appendChild(triggerBtn);
  triggerBtn.focus();
  assert.equal(document.activeElement, triggerBtn);

  const panel = createAgentChatPanel({
    isOpen: true,
    activeContext: null,
    messages: [],
    pending: false,
    draft: '',
    attachment: null,
    onClose: () => {
      closeTriggered = true;
    },
    onSend: () => {},
    onChangeDraft: () => {},
    onAttachClick: () => {},
    onRemoveAttachment: () => {},
  });
  document.body.appendChild(panel.el);

  // Focus was moved to composer textarea
  assert.equal(document.activeElement.tagName.toLowerCase(), 'textarea');

  // Dispatch Escape keydown event on window
  const escapeEv = new Event('keydown');
  escapeEv.key = 'Escape';
  escapeEv.preventDefault = () => {};
  window.dispatchEvent(escapeEv);
  assert.equal(closeTriggered, true, 'Escape key on window must invoke onClose callback');

  // Closing panel deactivates trap and restores focus
  panel.update({ isOpen: false });
  assert.equal(document.activeElement, triggerBtn, 'Focus must be restored to previous active element on panel close');

  panel.destroy();
  console.log('✓ 11. Escape key closes panel and restores focus to previous active element');
}

// 12. Layout constraints: 580px desktop, 100vw x 100dvh mobile, safe-area insets
{
  const cssPath = resolve(process.cwd(), 'frontend/styles/organisms/agent-chat-panel.css');
  const cssContent = readFileSync(cssPath, 'utf-8');

  // Desktop 580px constraint
  assert.ok(
    cssContent.includes('@media (min-width: 769px)') && cssContent.includes('width: 580px'),
    'Desktop CSS must enforce exactly 580px panel width'
  );

  // Mobile 100vw x 100dvh constraint
  assert.ok(
    cssContent.includes('@media (max-width: 768px)') &&
      cssContent.includes('width: 100vw') &&
      cssContent.includes('height: 100dvh'),
    'Mobile CSS must enforce 100vw and 100dvh fullscreen overlay'
  );

  // Safe area insets
  assert.ok(
    cssContent.includes('safe-area-inset-top') && cssContent.includes('safe-area-inset-bottom'),
    'Mobile CSS must account for safe-area insets'
  );

  console.log('✓ 12. Responsive layout verified (580px desktop, 100vw x 100dvh mobile, safe-area insets)');
}

// 13. Notice rendering with code fallback and retry button
{
  let retryClicked = false;
  const panel = createAgentChatPanel({
    isOpen: true,
    activeContext: null,
    messages: [
      {
        id: 'msg-u1',
        role: 'user',
        content: 'Crear show acústico',
      },
    ],
    pending: false,
    draft: 'Crear show acústico',
    attachment: null,
    notice: {
      code: 'provider_unavailable',
      retryable: true,
      // note: notice.message is missing!
    },
    onClose: () => {},
    onSend: () => {},
    onChangeDraft: () => {},
    onAttachClick: () => {},
    onRemoveAttachment: () => {},
    onRetry: () => {
      retryClicked = true;
    },
  });

  const noticeBanner = panel.el.querySelector('.agent-chat-notice-banner');
  assert.ok(noticeBanner !== null);
  assert.equal(noticeBanner.style.display, 'flex');

  const noticeText = panel.el.querySelector('.agent-chat-notice-text');
  assert.ok(noticeText !== null);
  assert.ok(
    noticeText.textContent.includes('proveedor de IA no está disponible'),
    'Notice text must be derived from notice.code when notice.message is absent'
  );

  const retryBtn = panel.el.querySelector('.agent-chat-retry-btn');
  assert.ok(retryBtn !== null, 'Actionable retry button must be rendered');
  retryBtn.dispatchEvent(new Event('click'));
  assert.equal(retryClicked, true, 'Clicking retry button must trigger onRetry callback');

  console.log('✓ 13. Notice rendered from notice.code fallback with actionable retry button');
}

console.log('All Agent Chat UI Components and Behaviors tests passed!');
