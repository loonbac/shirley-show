import assert from 'node:assert/strict';
import { setupDomEnvironment } from '../../tests/fake-dom.mjs';
import { renderMarkdown } from '../js/utils/markdown/render-markdown.js';

setupDomEnvironment();

console.log('Running render-markdown tests...');

if (!globalThis.HTMLElement.prototype.click) {
  globalThis.HTMLElement.prototype.click = function () {
    this.dispatchEvent(new Event('click'));
  };
}

/**
 * Renders markdown and returns the resulting nodes.
 * @param {string} text
 * @returns {Array<Node>}
 */
function render(text) {
  return renderMarkdown(text);
}

/** Serializes nodes to a comparable string, ignoring blank text nodes. */
function serialize(nodes) {
  return nodes
    .map((node) => serializeNode(node))
    .filter(Boolean)
    .join('');
}

function serializeNode(node) {
  if (node.nodeType === 3) {
    const text = node.textContent;
    return text.trim() === '' ? '' : text;
  }
  const tag = node.tagName.toLowerCase();
  const inner = node.childNodes
    .map((child) => serializeNode(child))
    .filter(Boolean)
    .join('');
  if (tag === 'a') {
    return `<a href="${node.getAttribute('href')}">${inner}</a>`;
  }
  return `<${tag}>${inner}</${tag}>`;
}

function findByTag(nodes, tag) {
  const found = [];
  const walk = (list) => {
    for (const node of list) {
      if (node.nodeType === 3) continue;
      if (node.tagName.toLowerCase() === tag) found.push(node);
      if (node.childNodes) walk(node.childNodes);
    }
  };
  walk(nodes);
  return found;
}

// 1. Plain text stays plain: no markup injected, just paragraph wrapping
{
  const nodes = render('Hola, Shirley.');
  assert.equal(serialize(nodes), '<p>Hola, Shirley.</p>');
  assert.equal(findByTag(nodes, 'script').length, 0);
}

// 2. Headings from #, ##, ### and ####
{
  const nodes = render('# Uno\n## Dos\n### Tres\n#### Cuatro');
  const h1 = findByTag(nodes, 'h1');
  const h2 = findByTag(nodes, 'h2');
  const h3 = findByTag(nodes, 'h3');
  const h4 = findByTag(nodes, 'h4');
  assert.equal(h1.length, 1, 'one h1');
  assert.equal(h2.length, 1, 'one h2');
  assert.equal(h3.length, 1, 'one h3');
  assert.equal(h4.length, 1, 'one h4');
  assert.equal(h1[0].textContent, 'Uno');
  assert.equal(h4[0].textContent, 'Cuatro');
}

// 3. Bold, italic, and both together
{
  const nodes = render('**negrita** y *cursiva* y ***las dos***');
  const strong = findByTag(nodes, 'strong');
  const em = findByTag(nodes, 'em');
  assert.equal(strong.length, 2, 'negrita and the bold half of "las dos"');
  assert.equal(strong[0].textContent, 'negrita');
  assert.equal(em[0].textContent, 'cursiva');
  assert.equal(strong[1].textContent, 'las dos', 'bold+italic nests inside strong');
  assert.equal(findByTag(strong[1].childNodes, 'em').length, 1, 'em nested inside the second strong');
}

// 4. Strikethrough and inline code
{
  const nodes = render('~~tachado~~ y `código`');
  const del = findByTag(nodes, 'del');
  const code = findByTag(nodes, 'code');
  assert.equal(del.length, 1);
  assert.equal(del[0].textContent, 'tachado');
  assert.equal(code.length, 1);
  assert.equal(code[0].textContent, 'código');
}

// 5. Bullet lists, including one level of nesting
{
  const nodes = render('- Uno\n- Dos\n  - Anidado');
  const lists = findByTag(nodes, 'ul');
  assert.ok(lists.length >= 1, 'a ul exists');
  const items = findByTag(nodes, 'li');
  assert.ok(items.length >= 3, 'three list items, one nested');
  const nested = lists.find((list) => list !== lists[0]);
  assert.ok(nested, 'nested list is inside the parent item, not a sibling list');
}

// 6. Ordered lists
{
  const nodes = render('1. Ensayar\n2. Revisar sonido');
  const ol = findByTag(nodes, 'ol');
  assert.equal(ol.length, 1);
  assert.equal(findByTag(nodes, 'li').length, 2);
}

// 7. Task lists become real checkboxes, checked state preserved
{
  const nodes = render('- [x] Letras cargadas\n- [ ] Pistas de audio');
  const checks = findByTag(nodes, 'input');
  assert.equal(checks.length, 2);
  assert.equal(checks[0].getAttribute('type'), 'checkbox');
  assert.equal(checks[0].hasAttribute('checked'), true, 'checked item stays checked');
  assert.equal(checks[1].hasAttribute('checked'), false, 'unchecked item stays unchecked');
}

// 8. Tables render with real th/td cells
{
  const nodes = render('| N° | Canción |\n|----|---------|\n| 1 | Como la flor |');
  const table = findByTag(nodes, 'table');
  assert.equal(table.length, 1);
  assert.equal(findByTag(nodes, 'th').length, 2, 'header cells');
  assert.equal(findByTag(nodes, 'td').length, 2, 'body cells');
  assert.equal(findByTag(nodes, 'th')[1].textContent, 'Canción');
  assert.equal(findByTag(nodes, 'td')[1].textContent, 'Como la flor');
}

// 9. Blockquotes
{
  const nodes = render('> La música empieza donde termina el lenguaje.');
  const quote = findByTag(nodes, 'blockquote');
  assert.equal(quote.length, 1);
  assert.ok(quote[0].textContent.includes('La música empieza'));
}

// 10. Fenced code blocks keep their content verbatim and never parse markdown inside
{
  const nodes = render('```python\ndef saludo(nombre):\n    return f"Hola, {nombre}"\n```');
  const blocks = findByTag(nodes, 'pre');
  assert.equal(blocks.length, 1, 'one pre block');
  const codeEls = findByTag(nodes, 'code');
  assert.ok(codeEls.length >= 1, 'code element inside pre');
  const text = blocks[0].textContent;
  assert.ok(text.includes('def saludo(nombre):'), 'verbatim body');
  assert.ok(text.includes('    return f"Hola, {nombre}"'), 'indentation preserved');
  assert.ok(!text.includes('```'), 'fence markers dropped');
}

// 11. Horizontal rules
{
  const nodes = render('uno\n\n---\n\ndos');
  assert.equal(findByTag(nodes, 'hr').length, 1);
}

// 12. Links render as anchors with safe hrefs
{
  const nodes = render('[Shirley Show](https://ejemplo.com)');
  const anchors = findByTag(nodes, 'a');
  assert.equal(anchors.length, 1);
  assert.equal(anchors[0].getAttribute('href'), 'https://ejemplo.com');
  assert.equal(anchors[0].getAttribute('rel'), 'noopener noreferrer');
  assert.equal(anchors[0].getAttribute('target'), '_blank');
}

// 13. Unsafe URLs never become clickable, including ones only a browser's
// own parser normalizes into a dangerous scheme.
{
  const badUrls = [
    'javascript:alert(1)',
    'JaVaScRiPt:alert(1)',
    'data:text/html,<script>alert(1)</script>',
    'data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==',
    'vbscript:msgbox(1)',
    'file:///etc/passwd',
    'blob:https://x/y',
    'about:blank',
    // Whitespace and control characters that a browser strips while parsing,
    // turning the raw text into a real javascript: URL.
    '\u0000javascript:alert(1)',
    'java\tscript:alert(1)',
    'java\nscript:alert(1)',
    '  javascript:alert(1)',
    '\tjavascript:alert(1)',
    '\njavascript:alert(1)',
  ];
  for (const bad of badUrls) {
    const nodes = render(`[x](${bad})`);
    const anchors = findByTag(nodes, 'a');
    assert.equal(anchors.length, 0, `${JSON.stringify(bad)} must not become a link`);
  }
}

// 13b. Safe destinations still work, including relative and mailto forms
{
  for (const good of ['https://ejemplo.com', 'http://x.y/z?a=1', 'mailto:a@b.c', 'tel:+51999', '/rel/path', '#ancla']) {
    const nodes = render(`[x](${good})`);
    const anchors = findByTag(nodes, 'a');
    assert.equal(anchors.length, 1, `${good} must stay clickable`);
    assert.equal(anchors[0].getAttribute('href'), good);
  }
}

// 14. Raw HTML is shown as literal text, never parsed into elements
{
  const nodes = render('<script>alert(1)</script> y <img src=x onerror=alert(1)>');
  assert.equal(findByTag(nodes, 'script').length, 0, 'no script element');
  assert.equal(findByTag(nodes, 'img').length, 0, 'no img element');
  assert.ok(serialize(nodes).includes('<script>'), 'shown as text');
}

// 15. Untrusted attributes cannot inject event handlers via image syntax
{
  const nodes = render('![x](javascript:alert(1))');
  const images = findByTag(nodes, 'img');
  assert.equal(images.length, 0, 'unsafe image src is not rendered as an img');
}

// 16. Empty and whitespace-only input yields no nodes
{
  assert.equal(render('').length, 0);
  assert.equal(render('   \n\n  ').length, 0);
  assert.equal(render(null).length, 0);
  assert.equal(render(undefined).length, 0);
}

// 17. Paragraphs are separated by blank lines
{
  const nodes = render('primero\n\nsegundo');
  const paras = findByTag(nodes, 'p');
  assert.equal(paras.length, 2);
  assert.equal(paras[0].textContent, 'primero');
  assert.equal(paras[1].textContent, 'segundo');
}

// 18. Single newline inside a paragraph becomes a line break, not a new paragraph
{
  const nodes = render('linea uno\nlinea dos');
  assert.equal(findByTag(nodes, 'p').length, 1, 'still one paragraph');
  assert.ok(findByTag(nodes, 'br').length >= 1, 'break inside the paragraph');
}
