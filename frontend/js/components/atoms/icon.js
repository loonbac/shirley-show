/**
 * Pure SVG Icon registry and creator.
 * @param {{
 *   name: 'play' | 'pause' | 'plus' | 'trash' | 'edit' | 'arrow-up' | 'arrow-down' | 'back' | 'track' | 'lyrics' | 'note' | 'check' | 'close' | 'grip' | 'sparkles' | 'send' | 'paperclip' | 'bot' | 'download' | 'camera' | 'ticket' | 'smartphone' | 'alert-triangle' | 'chevron-up' | 'chevron-down' | 'history',
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
    case 'sparkles':
      svg.innerHTML = '<path d="M12 2l2.4 7.2L22 12l-7.6 2.8L12 22l-2.4-7.2L2 12l7.6-2.8z"></path><path d="M18 2l1 3 3 1-3 1-1 3-1-3-3-1 3-1z"></path>';
      break;
    case 'send':
      svg.innerHTML = '<line x1="22" y1="2" x2="11" y2="13"></line><polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>';
      break;
    case 'paperclip':
      svg.innerHTML = '<path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"></path>';
      break;
    case 'bot':
      svg.innerHTML = '<rect x="3" y="11" width="18" height="10" rx="2"></rect><circle cx="12" cy="5" r="2"></circle><path d="M12 7v4"></path><line x1="8" y1="16" x2="8.01" y2="16"></line><line x1="16" y1="16" x2="16.01" y2="16"></line>';
      break;
    case 'download':
      svg.innerHTML = '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line>';
      break;
    case 'camera':
      svg.innerHTML = '<path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"></path><circle cx="12" cy="13" r="4"></circle>';
      break;
    case 'ticket':
      svg.innerHTML = '<path d="M2 9a3 3 0 0 1 0 6v2a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-2a3 3 0 0 1 0-6V7a2 2 0 0 0-2-2H4a2 2 0 0 0-2 2z"></path><line x1="13" y1="5" x2="13" y2="19"></line>';
      break;
    case 'smartphone':
      svg.innerHTML = '<rect x="5" y="2" width="14" height="20" rx="2" ry="2"></rect><line x1="12" y1="18" x2="12.01" y2="18"></line>';
      break;
    case 'alert-triangle':
      svg.innerHTML = '<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line>';
      break;
    case 'chevron-up':
      svg.innerHTML = '<polyline points="18 15 12 9 6 15"></polyline>';
      break;
    case 'chevron-down':
      svg.innerHTML = '<polyline points="6 9 12 15 18 9"></polyline>';
      break;
    case 'history':
      svg.innerHTML = '<circle cx="12" cy="12" r="10"></circle><polyline points="12 6 12 12 16 14"></polyline>';
      break;
    default:
      svg.innerHTML = '<circle cx="12" cy="12" r="10"></circle>';
  }

  return svg;
}
