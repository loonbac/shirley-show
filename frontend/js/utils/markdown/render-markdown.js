import { createElement } from '../dom/create-element.js';

/**
 * Minimal, dependency-free Markdown renderer for assistant chat replies.
 *
 * Design constraints:
 * - Returns DOM nodes, never an HTML string, so untrusted model output can never
 *   be parsed as markup. There is no innerHTML call anywhere in this module.
 * - Supports the subset the assistant actually emits: headings, emphasis,
 *   inline code, fenced code, lists (with nesting and task checkboxes),
 *   tables, blockquotes, horizontal rules, links, paragraphs.
 * - Link and image destinations are validated against a scheme allowlist, so
 *   javascript:, data:, and vbscript: URLs never become clickable.
 *
 * @module utils/markdown/render-markdown
 */

const SAFE_URL_SCHEMES = new Set(['http:', 'https:', 'mailto:', 'tel:']);

/**
 * Returns true when the URL is safe to place in an href/src attribute.
 * Relative URLs are allowed; absolute URLs must use an allowed scheme.
 * @param {string} url
 * @returns {boolean}
 */
export function isSafeUrl(url) {
  if (typeof url !== 'string') return false;
  const trimmed = url.trim();
  if (trimmed === '') return false;

  // Any explicit scheme must be allowlisted. A scheme is `word:` before any
  // path, query, or fragment separator.
  const schemeMatch = /^([a-zA-Z][a-zA-Z0-9+.-]*):/.exec(trimmed);
  if (!schemeMatch) {
    // No scheme: relative path, anchor, or query. Safe.
    return true;
  }

  let parsed;
  try {
    parsed = new URL(trimmed, 'https://example.invalid/');
  } catch {
    return false;
  }
  return SAFE_URL_SCHEMES.has(parsed.protocol.toLowerCase());
}

/** Inline markdown: emphasis, code, strikethrough, links, images. */
function renderInline(text) {
  const nodes = [];
  let buffer = '';

  const flush = () => {
    if (buffer !== '') {
      nodes.push(document.createTextNode(buffer));
      buffer = '';
    }
  };

  let i = 0;
  while (i < text.length) {
    const rest = text.slice(i);

    // Inline code first: its content is never parsed as markdown.
    if (rest.startsWith('`')) {
      const end = rest.indexOf('`', 1);
      if (end > 0) {
        flush();
        nodes.push(createElement('code', {}, [rest.slice(1, end)]));
        i += end + 1;
        continue;
      }
    }

    // Images: ![alt](url). Unsafe destinations degrade to the alt text.
    const imageMatch = /^!\[([^\]]*)\]\(([^)\s]+)\)/.exec(rest);
    if (imageMatch) {
      flush();
      const [, alt, url] = imageMatch;
      if (isSafeUrl(url)) {
        const img = createElement('img', { src: url, alt, loading: 'lazy' });
        nodes.push(img);
      } else {
        nodes.push(document.createTextNode(alt));
      }
      i += imageMatch[0].length;
      continue;
    }

    // Links: [text](url). Unsafe destinations degrade to plain text.
    const linkMatch = /^\[([^\]]*)\]\(([^)\s]+)\)/.exec(rest);
    if (linkMatch) {
      flush();
      const [, label, url] = linkMatch;
      if (isSafeUrl(url)) {
        nodes.push(
          createElement(
            'a',
            { href: url, target: '_blank', rel: 'noopener noreferrer' },
            renderInline(label)
          )
        );
      } else {
        nodes.push(document.createTextNode(label));
      }
      i += linkMatch[0].length;
      continue;
    }

    // Bold + italic: ***text*** or ___text___
    const boldItalic = /^(\*\*\*|___)(?=\S)([\s\S]*?\S)\1/.exec(rest);
    if (boldItalic) {
      flush();
      nodes.push(createElement('strong', {}, [createElement('em', {}, [boldItalic[2]])]));
      i += boldItalic[0].length;
      continue;
    }

    // Bold: **text** or __text__
    const bold = /^(\*\*|__)(?=\S)([\s\S]*?\S)\1/.exec(rest);
    if (bold) {
      flush();
      nodes.push(createElement('strong', {}, renderInline(bold[2])));
      i += bold[0].length;
      continue;
    }

    // Italic: *text* or _text_ (single delimiter, not part of a double).
    const italic = /^(\*|_)(?=\S)([\s\S]*?\S)\1(?!\1)/.exec(rest);
    if (italic) {
      flush();
      nodes.push(createElement('em', {}, renderInline(italic[2])));
      i += italic[0].length;
      continue;
    }

    // Strikethrough: ~~text~~
    const strike = /^~~(?=\S)([\s\S]*?\S)~~/.exec(rest);
    if (strike) {
      flush();
      nodes.push(createElement('del', {}, renderInline(strike[1])));
      i += strike[0].length;
      continue;
    }

    buffer += text[i];
    i += 1;
  }

  flush();
  return nodes;
}

/** Splits a table row into trimmed cell strings. */
function splitTableRow(line) {
  const trimmed = line.trim();
  const withoutEdges = trimmed.replace(/^\|/, '').replace(/\|$/, '');
  return withoutEdges.split('|').map((cell) => cell.trim());
}

/** True when the line is a table separator such as | --- | :--: | */
function isTableSeparator(line) {
  const trimmed = line.trim();
  if (!trimmed.includes('-')) return false;
  if (!/^[|:\-\s]+$/.test(trimmed)) return false;
  return splitTableRow(trimmed).every((cell) => /^:?-{1,}:?$/.test(cell));
}

/** Parses a task list marker, returning { checked } or null. */
function parseTaskMarker(text) {
  const match = /^\[( |x|X)\]\s+(.*)$/.exec(text);
  if (!match) return null;
  return { checked: match[1].toLowerCase() === 'x', label: match[2] };
}

const BULLET_RE = /^(\s*)[-*+]\s+(.*)$/;
const ORDERED_RE = /^(\s*)\d+[.)]\s+(.*)$/;

/** Renders one list starting at startIndex, consuming its items. */
function renderList(lines, startIndex, baseIndent, ordered) {
  const items = [];
  const matcher = ordered ? ORDERED_RE : BULLET_RE;
  let i = startIndex;

  while (i < lines.length) {
    const line = lines[i];
    const match = matcher.exec(line);
    if (!match) break;

    const indent = match[1].length;
    if (indent < baseIndent) break;
    if (indent > baseIndent) {
      // Deeper indent belongs to the previous item's nested list.
      const parent = items[items.length - 1];
      if (!parent) break;
      const nested = renderList(lines, i, indent, ordered);
      parent.appendChild(nested.list);
      i = nested.nextIndex;
      continue;
    }

    let content = match[2];
    const li = createElement('li');

    const task = parseTaskMarker(content);
    if (task) {
      const checkbox = createElement('input', { type: 'checkbox', disabled: true });
      if (task.checked) {
        checkbox.setAttribute('checked', '');
        checkbox.checked = true;
      }
      li.appendChild(checkbox);
      li.appendChild(document.createTextNode(' '));
      content = task.label;
    }

    li.appendChild(document.createTextNode(task ? '' : ''));
    for (const node of renderInline(content)) {
      li.appendChild(node);
    }
    items.push(li);
    i += 1;
  }

  const list = createElement(ordered ? 'ol' : 'ul', {}, items);
  return { list, nextIndex: i };
}

/**
 * Renders assistant markdown into a document fragment-safe array of nodes.
 * @param {string} source
 * @returns {Array<Node>}
 */
export function renderMarkdown(source) {
  if (typeof source !== 'string') return [];
  if (source.trim() === '') return [];

  const lines = source.replace(/\r\n?/g, '\n').split('\n');
  const nodes = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    // Blank line
    if (trimmed === '') {
      i += 1;
      continue;
    }

    // Fenced code block
    const fenceMatch = /^```([\w+-]*)\s*$/.exec(trimmed);
    if (fenceMatch) {
      const body = [];
      i += 1;
      while (i < lines.length && !/^```\s*$/.test(lines[i].trim())) {
        body.push(lines[i]);
        i += 1;
      }
      i += 1; // consume closing fence
      const codeEl = createElement('code', {}, [body.join('\n')]);
      if (fenceMatch[1]) {
        codeEl.setAttribute('data-lang', fenceMatch[1]);
      }
      nodes.push(createElement('pre', {}, [codeEl]));
      continue;
    }

    // Horizontal rule
    if (/^(\*\s*){3,}$/.test(trimmed) || /^(-\s*){3,}$/.test(trimmed) || /^(_\s*){3,}$/.test(trimmed)) {
      nodes.push(createElement('hr'));
      i += 1;
      continue;
    }

    // Heading
    const headingMatch = /^(#{1,6})\s+(.*)$/.exec(trimmed);
    if (headingMatch) {
      const level = headingMatch[1].length;
      nodes.push(createElement(`h${level}`, {}, renderInline(headingMatch[2])));
      i += 1;
      continue;
    }

    // Table: a header row followed by a separator row
    if (line.includes('|') && i + 1 < lines.length && isTableSeparator(lines[i + 1])) {
      const headerCells = splitTableRow(line);
      const thead = createElement('thead', {}, [
        createElement(
          'tr',
          {},
          headerCells.map((cell) => createElement('th', {}, renderInline(cell)))
        ),
      ]);
      i += 2;
      const bodyRows = [];
      while (i < lines.length && lines[i].includes('|') && lines[i].trim() !== '') {
        const cells = splitTableRow(lines[i]);
        bodyRows.push(
          createElement(
            'tr',
            {},
            cells.map((cell) => createElement('td', {}, renderInline(cell)))
          )
        );
        i += 1;
      }
      const children = [thead];
      if (bodyRows.length > 0) {
        children.push(createElement('tbody', {}, bodyRows));
      }
      nodes.push(createElement('table', {}, children));
      continue;
    }

    // Blockquote
    if (trimmed.startsWith('>')) {
      const quoted = [];
      while (i < lines.length && lines[i].trim().startsWith('>')) {
        quoted.push(lines[i].trim().replace(/^>\s?/, ''));
        i += 1;
      }
      nodes.push(createElement('blockquote', {}, renderMarkdown(quoted.join('\n'))));
      continue;
    }

    // Lists
    if (BULLET_RE.test(line) || ORDERED_RE.test(line)) {
      const ordered = ORDERED_RE.test(line);
      const indent = (ordered ? ORDERED_RE : BULLET_RE).exec(line)[1].length;
      const { list, nextIndex } = renderList(lines, i, indent, ordered);
      nodes.push(list);
      i = nextIndex;
      continue;
    }

    // Paragraph: consume until a blank line or the start of another block
    const paragraphLines = [];
    while (i < lines.length) {
      const current = lines[i];
      const currentTrimmed = current.trim();
      if (currentTrimmed === '') break;
      if (/^```/.test(currentTrimmed)) break;
      if (/^#{1,6}\s+/.test(currentTrimmed)) break;
      if (currentTrimmed.startsWith('>')) break;
      if (BULLET_RE.test(current) || ORDERED_RE.test(current)) break;
      if (/^(\*\s*){3,}$/.test(currentTrimmed) || /^(-\s*){3,}$/.test(currentTrimmed)) break;
      paragraphLines.push(current);
      i += 1;
    }

    if (paragraphLines.length > 0) {
      const paraChildren = [];
      paragraphLines.forEach((paragraphLine, index) => {
        if (index > 0) {
          paraChildren.push(createElement('br'));
        }
        paraChildren.push(...renderInline(paragraphLine));
      });
      nodes.push(createElement('p', {}, paraChildren));
    }
  }

  return nodes;
}
