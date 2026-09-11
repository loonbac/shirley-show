import { createElement } from '../../utils/dom/create-element.js';

/**
 * Creates an accessible button atom.
 * @param {{
 *   text?: string,
 *   variant?: 'primary' | 'secondary' | 'danger' | 'ghost',
 *   size?: 'base' | 'sm',
 *   type?: 'button' | 'submit' | 'reset',
 *   disabled?: boolean,
 *   fullWidth?: boolean,
 *   icon?: HTMLElement,
 *   onClick?: (e: MouseEvent) => void,
 * }} props 
 * @returns {{ el: HTMLButtonElement, update: (props: Partial<typeof props>) => void }}
 */
export function createButton(props = {}) {
  let {
    text = '',
    variant = 'primary',
    size = 'base',
    type = 'button',
    disabled = false,
    fullWidth = false,
    icon = null,
    onClick,
  } = props;

  const getClasses = () => [
    'btn',
    `btn-${variant}`,
    size === 'sm' ? 'btn-sm' : '',
    fullWidth ? 'btn-full' : '',
  ].filter(Boolean);

  const textSpan = createElement('span', {}, [text]);

  const el = createElement(
    'button',
    {
      type,
      class: getClasses(),
      disabled,
      onClick: (e) => {
        if (!el.disabled && typeof onClick === 'function') {
          onClick(e);
        }
      },
    },
    [icon, text ? textSpan : null].filter(Boolean)
  );

  function update(nextProps = {}) {
    if (nextProps.text !== undefined) {
      text = nextProps.text;
      textSpan.textContent = text;
    }
    if (nextProps.variant !== undefined) {
      variant = nextProps.variant;
      el.className = getClasses().join(' ');
    }
    if (nextProps.disabled !== undefined) {
      disabled = nextProps.disabled;
      el.disabled = disabled;
    }
    if (nextProps.onClick !== undefined) {
      onClick = nextProps.onClick;
    }
  }

  return { el, update };
}
