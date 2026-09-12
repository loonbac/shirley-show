export class FakeNode extends EventTarget {
  constructor(nodeName, nodeType) {
    super();
    this.nodeName = nodeName;
    this.nodeType = nodeType;
    this.parentNode = null;
    this.childNodes = [];
  }

  get firstChild() {
    return this.childNodes[0] || null;
  }

  get lastChild() {
    return this.childNodes[this.childNodes.length - 1] || null;
  }

  appendChild(child) {
    if (child.parentNode) {
      child.parentNode.removeChild(child);
    }
    child.parentNode = this;
    this.childNodes.push(child);
    return child;
  }

  removeChild(child) {
    const idx = this.childNodes.indexOf(child);
    if (idx !== -1) {
      this.childNodes.splice(idx, 1);
      child.parentNode = null;
      return child;
    }
    throw new Error('Child not found');
  }

  contains(other) {
    if (!other) return false;
    if (other === this) return true;
    let curr = other.parentNode;
    while (curr) {
      if (curr === this) return true;
      curr = curr.parentNode;
    }
    return false;
  }

  get textContent() {
    return this.childNodes.map((c) => c.textContent || '').join('');
  }

  set textContent(text) {
    this.childNodes = [new FakeTextNode(String(text))];
    this.childNodes[0].parentNode = this;
  }
}

export class FakeTextNode extends FakeNode {
  constructor(text) {
    super('#text', 3);
    this._text = String(text);
  }

  get textContent() {
    return this._text;
  }

  set textContent(v) {
    this._text = String(v);
  }
}

export class FakeElement extends FakeNode {
  constructor(tagName) {
    super(tagName.toUpperCase(), 1);
    this.tagName = tagName.toUpperCase();
    this.attributes = new Map();
    this.dataset = {};
    this.style = {};
    this.classList = {
      _classes: new Set(),
      add: (...cls) => {
        cls.forEach((c) => this.classList._classes.add(c));
        this._syncClass();
      },
      remove: (...cls) => {
        cls.forEach((c) => this.classList._classes.delete(c));
        this._syncClass();
      },
      contains: (c) => this.classList._classes.has(c),
      toggle: (c, force) => {
        if (force === true) this.classList.add(c);
        else if (force === false) this.classList.remove(c);
        else if (this.classList.contains(c)) this.classList.remove(c);
        else this.classList.add(c);
      },
    };
    this._innerHTML = '';
  }

  _syncClass() {
    const joined = Array.from(this.classList._classes).join(' ');
    this.attributes.set('class', joined);
  }

  get className() {
    return this.getAttribute('class') || '';
  }

  set className(val) {
    this.setAttribute('class', val);
    this.classList._classes = new Set(String(val).split(/\s+/).filter(Boolean));
  }

  get children() {
    return this.childNodes.filter((n) => n.nodeType === 1);
  }

  setAttribute(name, value) {
    this.attributes.set(name, String(value));
    if (name === 'class') {
      this.classList._classes = new Set(String(value).split(/\s+/).filter(Boolean));
    }
  }

  getAttribute(name) {
    return this.attributes.has(name) ? this.attributes.get(name) : null;
  }

  hasAttribute(name) {
    return this.attributes.has(name);
  }

  removeAttribute(name) {
    this.attributes.delete(name);
    if (name === 'class') {
      this.classList._classes.clear();
    }
  }

  get innerHTML() {
    return this._innerHTML || this.textContent;
  }

  set innerHTML(html) {
    this._innerHTML = html;
    if (html === '') {
      for (const child of this.childNodes) {
        child.parentNode = null;
      }
      this.childNodes = [];
    }
  }

  focus() {
    if (globalThis.document) {
      globalThis.document.activeElement = this;
    }
  }

  blur() {
    if (globalThis.document && globalThis.document.activeElement === this) {
      globalThis.document.activeElement = globalThis.document.body;
    }
  }

  querySelector(selector) {
    return this.querySelectorAll(selector)[0] || null;
  }

  querySelectorAll(selector) {
    const matches = [];
    function traverse(el) {
      for (const child of el.children) {
        if (matchesSelector(child, selector)) {
          matches.push(child);
        }
        traverse(child);
      }
    }
    traverse(this);
    return matches;
  }
}

function matchesSelector(el, selector) {
  if (selector.includes(',')) {
    return selector.split(',').some((s) => matchesSelector(el, s.trim()));
  }
  if (selector.endsWith(':not([disabled])')) {
    const base = selector.slice(0, -':not([disabled])'.length);
    return matchesSelector(el, base) && !el.hasAttribute('disabled');
  }
  if (selector.startsWith('.')) {
    return el.classList.contains(selector.slice(1));
  }
  if (selector.startsWith('#')) {
    return el.getAttribute('id') === selector.slice(1);
  }
  if (selector.startsWith('[') && selector.endsWith(']')) {
    const attr = selector.slice(1, -1);
    if (attr.includes('=')) {
      const [k, v] = attr.split('=');
      const cleanVal = v.replace(/^['"]|['"]$/g, '');
      return el.getAttribute(k) === cleanVal;
    }
    return el.hasAttribute(attr);
  }
  return el.tagName.toLowerCase() === selector.toLowerCase();
}

export class FakeDocument extends FakeElement {
  constructor() {
    super('#document');
    this.body = new FakeElement('body');
    this.documentElement = new FakeElement('html');
    this.appendChild(this.documentElement);
    this.documentElement.appendChild(this.body);
    this.activeElement = this.body;
  }

  createElement(tag) {
    return new FakeElement(tag);
  }

  createElementNS(ns, tag) {
    return new FakeElement(tag);
  }

  createTextNode(text) {
    return new FakeTextNode(text);
  }
}

export class FakeWindow extends EventTarget {
  constructor() {
    super();
    this.matchMedia = () => ({ matches: false });
  }
}

export function setupDomEnvironment() {
  globalThis.Node = FakeNode;
  globalThis.HTMLElement = FakeElement;
  globalThis.SVGElement = FakeElement;
  globalThis.document = new FakeDocument();
  globalThis.window = new FakeWindow();
  if (!globalThis.CustomEvent) {
    globalThis.CustomEvent = class CustomEvent extends Event {
      constructor(type, init = {}) {
        super(type);
        this.detail = init.detail || null;
      }
    };
  }
}
