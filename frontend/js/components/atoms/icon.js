/**
 * Pure SVG Icon registry and creator.
 * @param {{
 *   name: 'play' | 'pause' | 'plus' | 'trash' | 'edit' | 'arrow-up' | 'arrow-down' | 'back' | 'track' | 'lyrics' | 'note' | 'check' | 'close' | 'grip',
 *   size?: 'sm' | 'base' | 'lg' | 'xl',
 *   className?: string,
 * }} props 
 * @returns {SVGElement}
 */
export function createIcon(props = {}) {
  const { name, size = 'base', className = '' } = props;

  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('fill', 'none');
  svg.setAttribute('stroke', 'currentColor');
  svg.setAttribute('stroke-width', '2');
  svg.setAttribute('stroke-linecap', 'round');
  svg.setAttribute('stroke-linejoin', 'round');
  svg.setAttribute('aria-hidden', 'true');

  const sizeClass = size === 'base' ? '' : `icon-${size}`;
  svg.setAttribute('class', ['icon', sizeClass, className].filter(Boolean).join(' '));

  switch (name) {
    case 'grip':
      svg.innerHTML = '<circle cx="9" cy="5" r="1.5" fill="currentColor"></circle><circle cx="9" cy="12" r="1.5" fill="currentColor"></circle><circle cx="9" cy="19" r="1.5" fill="currentColor"></circle><circle cx="15" cy="5" r="1.5" fill="currentColor"></circle><circle cx="15" cy="12" r="1.5" fill="currentColor"></circle><circle cx="15" cy="19" r="1.5" fill="currentColor"></circle>';
      break;
    case 'play':
      svg.innerHTML = '<polygon points="5 3 19 12 5 21 5 3" fill="currentColor"></polygon>';
      break;
    case 'pause':
      svg.innerHTML = '<rect x="6" y="4" width="4" height="16" fill="currentColor"></rect><rect x="14" y="4" width="4" height="16" fill="currentColor"></rect>';
      break;
    case 'plus':
      svg.innerHTML = '<line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line>';
      break;
    case 'trash':
      svg.innerHTML = '<polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>';
      break;
    case 'edit':
      svg.innerHTML = '<path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path>';
      break;
    case 'arrow-up':
      svg.innerHTML = '<line x1="12" y1="19" x2="12" y2="5"></line><polyline points="5 12 12 5 19 12"></polyline>';
      break;
    case 'arrow-down':
      svg.innerHTML = '<line x1="12" y1="5" x2="12" y2="19"></line><polyline points="19 12 12 19 5 12"></polyline>';
      break;
    case 'back':
      svg.innerHTML = '<line x1="19" y1="12" x2="5" y2="12"></line><polyline points="12 19 5 12 12 5"></polyline>';
      break;
    case 'track':
      svg.innerHTML = '<circle cx="12" cy="12" r="10"></circle><polygon points="10 8 16 12 10 16 10 8" fill="currentColor"></polygon>';
      break;
    case 'lyrics':
      svg.innerHTML = '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline><line x1="16" y1="13" x2="8" y2="13"></line><line x1="16" y1="17" x2="8" y2="17"></line><polyline points="10 9 9 9 8 9"></polyline>';
      break;
    case 'note':
      svg.innerHTML = '<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"></path>';
      break;
    case 'check':
      svg.innerHTML = '<polyline points="20 6 9 17 4 12"></polyline>';
      break;
    case 'close':
      svg.innerHTML = '<line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line>';
      break;
    default:
      svg.innerHTML = '<circle cx="12" cy="12" r="10"></circle>';
  }

  return svg;
}
