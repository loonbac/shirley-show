import { createElement } from '../../utils/dom/create-element.js';

/**
 * Creates a mobile-accessible slider/range input atom (touch target >= 44px).
 * @param {{
 *   min?: number,
 *   max?: number,
 *   value?: number,
 *   step?: number,
 *   ariaLabel?: string,
 *   disabled?: boolean,
 *   onInput?: (val: number, e: Event) => void,
 *   onChange?: (val: number, e: Event) => void,
 * }} props 
 * @returns {{
 *   el: HTMLElement,
 *   setValue: (val: number) => void,
 *   setMax: (max: number) => void,
 *   setDisabled: (disabled: boolean) => void,
 * }}
 */
export function createRangeInput(props = {}) {
  const {
    min = 0,
    max = 100,
    value = 0,
    step = 1,
    ariaLabel = 'Progreso de audio',
    disabled = false,
    onInput,
    onChange,
  } = props;

  const rangeEl = createElement('input', {
    type: 'range',
    min,
    max,
    value,
    step,
    disabled,
    'aria-label': ariaLabel,
    class: 'range-input',
    onInput: (e) => {
      if (typeof onInput === 'function') {
        onInput(Number(rangeEl.value), e);
      }
    },
    onChange: (e) => {
      if (typeof onChange === 'function') {
        onChange(Number(rangeEl.value), e);
      }
    },
  });

  const el = createElement('div', { class: 'range-input-wrapper' }, [rangeEl]);

  function setValue(val) {
    rangeEl.value = String(val);
  }

  function setMax(newMax) {
    rangeEl.max = String(newMax);
  }

  function setDisabled(isDisabled) {
    rangeEl.disabled = isDisabled;
  }

  return { el, setValue, setMax, setDisabled };
}
