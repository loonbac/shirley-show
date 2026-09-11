import { createElement } from '../../utils/dom/create-element.js';
import { createButton } from '../atoms/button.js';
import { createIcon } from '../atoms/icon.js';
import { createIconButton } from '../molecules/icon-button.js';
import { formatRelativeDate } from '../../utils/format/relative-date.js';

/**
 * Creates an enriched catalog presentation card organism.
 * Displays title, relative date, item counts with color-coded badges,
 * an ordered setlist preview, discrete secondary actions, and a dominant "Abrir show" button.
 *
 * @param {{
 *   presentation: {
 *     id: string,
 *     title: string,
 *     created_at: string,
 *     updated_at: string,
 *     tracks_count?: number,
 *     lyrics_count?: number,
 *     notes_count?: number,
 *     total_items?: number,
 *     preview_items?: Array<{
 *       type: 'track' | 'lyrics' | 'note',
 *       id: string,
 *       position: number,
 *       title?: string,
 *       snippet?: string,
 *     }>,
 *   },
 *   onSelect: (id: string) => void,
 *   onRename: (id: string, currentTitle: string) => void,
 *   onDelete: (id: string, title: string) => void,
 * }} props 
 * @returns {HTMLElement}
 */
export function createPresentationCard(props) {
  const { presentation, onSelect, onRename, onDelete } = props;

  const tracksCount = Number(presentation.tracks_count) || 0;
  const lyricsCount = Number(presentation.lyrics_count) || 0;
  const notesCount = Number(presentation.notes_count) || 0;
  const totalItems = Number(presentation.total_items) || (tracksCount + lyricsCount + notesCount);
  const previewItems = Array.isArray(presentation.preview_items) ? presentation.preview_items : [];
  const formattedDate = formatRelativeDate(presentation.updated_at || presentation.created_at);

  // 1. Header Area: Title + discrete secondary menu
  const titleLink = createElement(
    'a',
    {
      href: `#/presentations/${encodeURIComponent(presentation.id)}`,
      class: 'presentation-card-title-link',
      onClick: (e) => {
        e.preventDefault();
        onSelect(presentation.id);
      },
    },
    [presentation.title]
  );

  const renameBtn = createIconButton({
    iconName: 'edit',
    ariaLabel: `Renombrar ${presentation.title}`,
    onClick: () => onRename(presentation.id, presentation.title),
  });

  const deleteBtn = createIconButton({
    iconName: 'trash',
    ariaLabel: `Eliminar ${presentation.title}`,
    variant: 'danger',
    onClick: () => onDelete(presentation.id, presentation.title),
  });

  const discreteActions = createElement('div', { class: 'presentation-card-discrete-actions' }, [
    renameBtn.el,
    deleteBtn.el,
  ]);

  const headerRow = createElement('div', { class: 'presentation-card-header-row' }, [\
    titleLink,
    discreteActions,
  ]);

  const dateEl = createElement('span', { class: 'presentation-card-date' }, [formattedDate]);

  const headerEl = createElement('header', { class: 'presentation-card-header' }, [
    headerRow,
    dateEl,
  ]);

  // 2. Badges Bar: Counts per item type with distinct semantic jewel tones
  const trackBadge = createElement(
    'span',
    { class: 'presentation-card-badge badge-track', title: `${tracksCount} pistas de audio` },
    [
      createIcon({ name: 'track', size: 'sm' }),
      createElement('span', { class: 'badge-count' }, [tracksCount]),
      createElement('span', { class: 'badge-label' }, [tracksCount === 1 ? 'pista' : 'pistas']),
    ]
  );

  const lyricsBadge = createElement(
    'span',
    { class: 'presentation-card-badge badge-lyrics', title: `${lyricsCount} letras` },
    [
      createIcon({ name: 'lyrics', size: 'sm' }),
      createElement('span', { class: 'badge-count' }, [lyricsCount]),
      createElement('span', { class: 'badge-label' }, [lyricsCount === 1 ? 'letra' : 'letras']),
    ]
  );

  const noteBadge = createElement(
    'span',
    { class: 'presentation-card-badge badge-note', title: `${notesCount} notas de escenario` },
    [
      createIcon({ name: 'note', size: 'sm' }),
      createElement('span', { class: 'badge-count' }, [notesCount]),
      createElement('span', { class: 'badge-label' }, [notesCount === 1 ? 'nota' : 'notas']),
    ]
  );

  const badgesBar = createElement('div', { class: 'presentation-card-badges' }, [
    trackBadge,
    lyricsBadge,
    noteBadge,
  ]);

  // 3. Setlist Preview Box: First 3 items or explanatory empty message
  const previewBox = createElement('div', { class: 'presentation-card-preview-box' });

  if (previewItems.length > 0) {
    const previewList = createElement('ol', { class: 'presentation-card-preview-list' });
    for (const item of previewItems) {
      const iconName = item.type === 'track' ? 'track' : (item.type === 'lyrics' ? 'lyrics' : 'note');
      const itemIcon = createIcon({ name: iconName, size: 'sm', className: `preview-icon preview-icon-${item.type}` });
      const itemText = item.type === 'track' ? (item.title || 'Pista sin título') : (item.snippet || 'Texto...');
      const itemLabel = createElement('span', { class: 'preview-item-label' }, [itemText]);

      const itemLi = createElement(
        'li',
        { class: `presentation-card-preview-item preview-item-${item.type}` },
        [itemIcon, itemLabel]
      );
      previewList.appendChild(itemLi);
    }

    if (totalItems > previewItems.length) {
      const remainingCount = totalItems - previewItems.length;
      const moreEl = createElement('li', { class: 'presentation-card-preview-more' }, [
        `+${remainingCount} ${remainingCount === 1 ? 'elemento más' : 'elementos más'}...`,
      ]);
      previewList.appendChild(moreEl);
    }

    previewBox.appendChild(previewList);
  } else {
    const emptyPreview = createElement('div', { class: 'presentation-card-preview-empty' }, [
      createElement('p', {}, ['Repertorio sin elementos aún']),
    ]);
    previewBox.appendChild(emptyPreview);
  }

  // 4. Primary Dominant Action: "Abrir show" with touch target >= 48px
  const openBtn = createButton({
    text: 'Abrir show',
    variant: 'primary',
    fullWidth: true,
    icon: createIcon({ name: 'play', size: 'base' }),
    onClick: () => onSelect(presentation.id),
  });
  openBtn.el.classList.add('presentation-card-open-btn');

  const footerEl = createElement('footer', { class: 'presentation-card-footer' }, [
    openBtn.el,
  ]);

  return createElement('li', { class: 'presentation-card' }, [
    headerEl,
    badgesBar,
    previewBox,
    footerEl,
  ]);
}
