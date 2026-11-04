import assert from 'node:assert/strict';
import { setupDomEnvironment } from '../../tests/fake-dom.mjs';
import { createAgentMessageBubble } from '../js/components/molecules/agent-message-bubble.js';

setupDomEnvironment();

console.log('Running agent-message-bubble markdown tests...');

if (!globalThis.HTMLElement.prototype.click) {
  globalThis.HTMLElement.prototype.click = function () {
    this.dispatchEvent(new Event('click'));
  };
}

/** Collects tag names present in a subtree. */
function tags(root) {
  const found = [];
  const walk = (node) => {
    for (const child of node.childNodes || []) {
      if (child.nodeType === 3) continue;
      found.push(child.tagName.toLowerCase());
      walk(child);
    }
  };
  walk(root);
  return found;
}

// 1. Assistant markdown becomes real elements
{
  const bubble = createAgentMessageBubble({
    message: { id: 'a1', role: 'assistant', content: '## Título\n\n- uno\n- dos' },
  });
  const found = tags(bubble.el);
  assert.ok(found.includes('h2'), 'heading rendered');
  assert.ok(found.includes('ul'), 'list rendered');
  assert.ok(found.includes('li'), 'list items rendered');
  assert.ok(
    bubble.el.querySelector('.agent-message-text--markdown'),
    'assistant body carries the markdown modifier class'
  );
}

// 2. User messages stay plain text, markdown syntax is not interpreted
{
  const bubble = createAgentMessageBubble({
    message: { id: 'u1', role: 'user', content: '## no soy un título' },
  });
  const found = tags(bubble.el);
  assert.ok(!found.includes('h2'), 'no heading in a user bubble');
  assert.equal(
    bubble.el.querySelector('.agent-message-text--markdown'),
    null,
    'user body has no markdown modifier class'
  );
  assert.ok(
    bubble.el.textContent.includes('## no soy un título'),
    'literal text preserved for the user'
  );
}

// 3. Raw HTML from the model never becomes an element
{
  const bubble = createAgentMessageBubble({
    message: {
      id: 'a2',
      role: 'assistant',
      content: '<img src=x onerror=alert(1)> <script>alert(2)</script>',
    },
  });
  const found = tags(bubble.el);
  assert.ok(!found.includes('script'), 'no script element');
  assert.ok(!found.includes('img'), 'no img element');
  assert.ok(bubble.el.textContent.includes('<script>'), 'shown as literal text');
}

// 4. update() re-renders markdown instead of flattening to text
{
  const bubble = createAgentMessageBubble({
    message: { id: 'a3', role: 'assistant', content: 'texto plano' },
  });
  bubble.update({
    message: { id: 'a3', role: 'assistant', content: '### Nuevo encabezado' },
  });
  assert.ok(tags(bubble.el).includes('h3'), 'markdown applied after update');
  assert.ok(!bubble.el.textContent.includes('###'), 'markers consumed, not shown');
}

// 5. update() does not duplicate previous content
{
  const bubble = createAgentMessageBubble({
    message: { id: 'a4', role: 'assistant', content: 'primero' },
  });
  bubble.update({ message: { id: 'a4', role: 'assistant', content: 'segundo' } });
  const text = bubble.el.textContent;
  assert.ok(text.includes('segundo'), 'new content present');
  assert.ok(!text.includes('primero'), 'old content replaced, not appended');
}

// 6. Empty assistant content renders an empty body without throwing
{
  const bubble = createAgentMessageBubble({
    message: { id: 'a5', role: 'assistant', content: '' },
  });
  assert.equal(bubble.el.querySelector('.agent-message-text').textContent, '');
}
