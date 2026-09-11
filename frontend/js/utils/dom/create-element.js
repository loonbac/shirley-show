/**
 * Pure DOM creation utility.
 * @param {string} tag 
 * @param {Record<string, any>} [props={}] 
 * @param {Array<Node|string|number>|Node|string|number} [children=[]] 
 * @returns {HTMLElement}
 */
export function createElement(tag, props = {}, children = []) {
  const el = document.createElement(tag);

  for (const [key, value] of Object.entries(props)) {
    if (value === undefined || value === null) continue;

    if (key === 'class' || key === 'className') {
      if (Array.isArray(value)) {
        el.className = value.filter(Boolean).join(' ');
      } else if (value) {
        el.className = String(value);
      }
    } else if (key === 'dataset' && typeof value === 'object') {
      for (const [dataKey, dataVal] of Object.entries(value)) {
        if (dataVal !== undefined && dataVal !== null) {
          el.dataset[dataKey] = String(dataVal);
        }
      }
    } else if (key.startsWith('on') && typeof value === 'function') {
      const eventName = key.slice(2).toLowerCase();
      el.addEventListener(eventName, value);
    } else if (key === 'style' && typeof value === 'object') {
      Object.assign(el.style, value);
    } else if (typeof value === 'boolean') {
      if (value) {
        el.setAttribute(key, '');
      } else {
        el.removeAttribute(key);
      }
      if (key in el) {
        el[key] = value;
      }
    } else {
      el.setAttribute(key, String(value));
      if (key in el && key !== 'type') {
        try {
          el[key] = value;
        } catch {
          // ignore read-only properties
        }
      }
    }
  }

  const childList = Array.isArray(children) ? children : [children];
  for (const child of childList) {
    if (child === null || child === undefined || child === false) continue;
    if (child instanceof Node) {
      el.appendChild(child);
    } else {
      el.appendChild(document.createTextNode(String(child)));
    }
  }

  return el;
}
