import { createElement } from '../../utils/dom/create-element.js';
import { createIcon } from '../atoms/icon.js';

/**
 * Creates an accessible icon button molecule (min-target 44x44px).
 * @param {{
 *   iconName: 'play' | 'pause' | 'plus' | 'trash' | 'edit' | 'arrow-up' | 'arrow-down' | 'back' | 'close',
 *   ariaLabel: string,
 *   title?: string,
 *   variant?: 'default' | 'danger' | 'primary',
 *   disabled?: boolean,
 *   onClick?: (e: MouseEvent) => void,
 * }} props 
 * @returns {{
 *   el: HTMLButtonElement,
 *   update: (props: { disabled?: boolean, iconName?: string, ariaLabel?: string }) => void,
 * }}
 */
export function createIconButton(props) {
  let {
    iconName,
    ariaLabel,
    title = '',
    variant = 'default',
    disabled = false,
    onClick,
  } = props;

  let currentIcon = createIcon({ name: iconName });

  const getClasses = () => [
    'icon-btn',
    variant === 'default' ? '' : `icon-btn-${variant}`,
  ].filter(Boolean);

  const el = createElement(
    'button',
    {
      type: 'button',
      class: getClasses(),
      title: title || ariaLabel,
      'aria-label': ariaLabel,
      disabled,
      onClick: (e) => {
        if (!el.disabled && typeof onClick === 'function') {
          onClick(e);
        }
      },
    },
    [currentIcon]
  );

  function update(nextProps = {}) {
    if (nextProps.disabled !== undefined) {
      disabled = nextProps.disabled;
      el.disabled = disabled;
    }
    if (nextProps.ariaLabel !== undefined) {
      ariaLabel = nextProps.ariaLabel;
      el.setAttribute('aria-label', ariaLabel);
      el.title = title || ariaLabel;
    }
    if (nextProps.iconName !== undefined && nextProps.iconName !== iconName) {
      iconName = nextProps.iconName;
      el.innerHTML = '';
      currentIcon = createIcon({ name: iconName });
      el.appendChild(currentIcon);
    }
  }

  return { el, update };
}
