import { createElement } from '../../utils/dom/create-element.js';

/**
 * Creates a controlled multiline text area atom with label and validation hint.
 * @param {{
 *   label?: string,
 *   name?: string,
 *   value?: string,
 *   placeholder?: string,
 *   required?: boolean,
 *   rows?: number,
 *   error?: string | null,
 *   onInput?: (value: string, e: Event) => void,
 *   onChange?: (value: string, e: Event) => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   getValue: () => string,
 *   setValue: (val: string) => void,
 *   setError: (err: string | null) => void,
 *   focus: () => void,
 * }}
 */
export function createTextArea(props = {}) {
  const {
    label = '',
    name = '',
    value = '',
    placeholder = '',
    required = false,
    rows = 4,
    error = null,
    onInput,
    onChange,
  } = props;

  const areaEl = createElement('textarea', {
    name,
    rows,
    placeholder,
    required,
    class: ['text-area', error ? 'text-area-error' : ''].filter(Boolean),
    onInput: (e) => {
      if (typeof onInput === 'function') {
        onInput(areaEl.value, e);
      }
    },
    onChange: (e) => {
      if (typeof onChange === 'function') {
        onChange(areaEl.value, e);
      }
    },
  }, [value]);

  const hintEl = createElement(
    'span',
    { class: 'text-area-hint', style: { display: error ? 'block' : 'none' } },
    [error || '']
  );

  const children = [];
  if (label) {
    children.push(createElement('label', { class: 'text-area-label' }, [label]));
  }
  children.push(areaEl);
  children.push(hintEl);

  const el = createElement('div', { class: 'text-area-wrapper' }, children);

  function getValue() {
    return areaEl.value;
  }

  function setValue(val) {
    areaEl.value = val;
  }

  function setError(err) {
    if (err) {
      areaEl.classList.add('text-area-error');
      hintEl.textContent = err;
      hintEl.style.display = 'block';
    } else {
      areaEl.classList.remove('text-area-error');
      hintEl.textContent = '';
      hintEl.style.display = 'none';
    }
  }

  function focus() {
    areaEl.focus();
  }

  return { el, getValue, setValue, setError, focus };
}
