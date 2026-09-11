import { createElement } from '../../utils/dom/create-element.js';

/**
 * Creates a controlled text input atom with label and validation hint.
 * @param {{
 *   label?: string,
 *   name?: string,
 *   value?: string,
 *   placeholder?: string,
 *   required?: boolean,
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
export function createTextInput(props = {}) {
  const {
    label = '',
    name = '',
    value = '',
    placeholder = '',
    required = false,
    error = null,
    onInput,
    onChange,
  } = props;

  const inputEl = createElement('input', {
    type: 'text',
    name,
    value,
    placeholder,
    required,
    class: ['text-input', error ? 'text-input-error' : ''].filter(Boolean),
    onInput: (e) => {
      if (typeof onInput === 'function') {
        onInput(inputEl.value, e);
      }
    },
    onChange: (e) => {
      if (typeof onChange === 'function') {
        onChange(inputEl.value, e);
      }
    },
  });

  const hintEl = createElement(
    'span',
    { class: 'text-input-hint', style: { display: error ? 'block' : 'none' } },
    [error || '']
  );

  const children = [];
  if (label) {
    children.push(createElement('label', { class: 'text-input-label' }, [label]));
  }
  children.push(inputEl);
  children.push(hintEl);

  const el = createElement('div', { class: 'text-input-wrapper' }, children);

  function getValue() {
    return inputEl.value;
  }

  function setValue(val) {
    inputEl.value = val;
  }

  function setError(err) {
    if (err) {
      inputEl.classList.add('text-input-error');
      hintEl.textContent = err;
      hintEl.style.display = 'block';
    } else {
      inputEl.classList.remove('text-input-error');
      hintEl.textContent = '';
      hintEl.style.display = 'none';
    }
  }

  function focus() {
    inputEl.focus();
  }

  return { el, getValue, setValue, setError, focus };
}
