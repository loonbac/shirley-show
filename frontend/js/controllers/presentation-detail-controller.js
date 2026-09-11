import { createAppTopbar } from '../components/organisms/app-topbar.js';
import { createButton } from '../components/atoms/button.js';
import { createIcon } from '../components/atoms/icon.js';
import { createPresentationDetailView } from '../components/views/presentation-detail-view.js';
import { createPresentationFormDialog } from '../components/organisms/presentation-form-dialog.js';
import { createAddItemDialog } from '../components/organisms/add-item-dialog.js';
import { createConfirmationDialog } from '../components/molecules/confirmation-dialog.js';
import { getPresentation } from '../data/get-presentation.js';
import { renamePresentation } from '../data/rename-presentation.js';
import { deletePresentation } from '../data/delete-presentation.js';
import { addTextItem } from '../data/add-text-item.js';
import { uploadTrack } from '../data/upload-track.js';
import { moveItem } from '../data/move-item.js';
import { deleteItem } from '../data/delete-item.js';
import { toPresentations } from '../router/navigate.js';
import { playbackSignal } from '../state/playback-state.js';

/**
 * Controller orchestrating a presentation's vertical timeline and exclusive playback.
 * @param {{
 *   presentationId: string,
 *   audioController: {
 *     play: (track: any) => void,
 *     pause: () => void,
 *     toggle: (track: any) => void,
 *     seek: (time: number) => void,
 *     stop: () => void,
 *   },
 * }} deps 
 * @returns {{
 *   topbarEl: HTMLElement,
 *   viewEl: HTMLElement,
 *   destroy: () => void,
 * }}
 */
export function createPresentationDetailController(deps) {
  const { presentationId, audioController } = deps;

  let presentation = null;
  let items = [];
  let isLoading = true;
  let errorMessage = null;
  let activeDialog = null;

  function closeActiveDialog() {
    if (activeDialog) {
      activeDialog.destroy();
      activeDialog = null;
    }
  }

  const addItemBtn = createButton({
    text: 'Agregar Elemento',
    variant: 'primary',
    icon: createIcon({ name: 'plus', size: 'sm' }),
    onClick: () => openAddItemDialog(),
  });

  const topbar = createAppTopbar({
    title: 'Cargando...',
    onBack: () => {
      audioController.stop();
      toPresentations();
    },
    actions: [addItemBtn.el],
  });

  const view = createPresentationDetailView({
    presentation,
    items,
    isLoading,
    errorMessage,
    playbackState: playbackSignal.get(),
    onRename: () => openRenameDialog(),
    onDeletePresentation: () => openDeletePresentationDialog(),
    onAddItemClick: () => openAddItemDialog(),
    onTogglePlay: (track) => {
      audioController.toggle(track);
    },
    onSeek: (seconds) => {
      audioController.seek(seconds);
    },
    onMoveUp: (item) => handleMoveItem(item, 'up'),
    onMoveDown: (item) => handleMoveItem(item, 'down'),
    onDeleteItem: (item) => openDeleteItemDialog(item),
  });

  // Subscribe to playback changes to update playback UI in real time
  const unsubscribePlayback = playbackSignal.subscribe((newPlaybackState) => {
    view.update({ playbackState: newPlaybackState });
  });

  async function loadData() {
    isLoading = true;
    errorMessage = null;
    view.update({ isLoading: true, errorMessage: null });

    try {
      const data = await getPresentation(presentationId);
      presentation = data.presentation;
      items = data.items || [];
      isLoading = false;

      topbar.update({ title: presentation.title });
      view.update({ presentation, items, isLoading: false });
    } catch (err) {
      isLoading = false;
      errorMessage = err.message || 'Error al cargar la presentación.';
      view.update({ isLoading: false, errorMessage });

      // Stale or missing presentation route handling
      if (err.status === 404 || err.code === 'presentation_not_found') {
        alert('La presentación solicitada no existe o fue eliminada.');
        audioController.stop();
        toPresentations();
      }
    }
  }

  function openRenameDialog() {
    if (!presentation) return;
    closeActiveDialog();
    activeDialog = createPresentationFormDialog({
      mode: 'rename',
      initialTitle: presentation.title,
      onSubmit: async (newTitle) => {
        try {
          const res = await renamePresentation(presentationId, { title: newTitle });
          presentation = res.presentation;
          topbar.update({ title: presentation.title });
          view.update({ presentation });
          closeActiveDialog();
        } catch (err) {
          alert(err.message || 'Error al renombrar la presentación.');
        }
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  function openDeletePresentationDialog() {
    if (!presentation) return;
    closeActiveDialog();
    activeDialog = createConfirmationDialog({
      title: '¿Eliminar presentación?',
      message: `¿Estás seguro de que deseas eliminar "${presentation.title}"? Esta acción no se puede deshacer y detendrá cualquier audio en reproducción.`,
      confirmText: 'Eliminar Presentación',
      isDanger: true,
      onConfirm: async () => {
        try {
          audioController.stop();
          await deletePresentation(presentationId);
          closeActiveDialog();
          toPresentations();
        } catch (err) {
          alert(err.message || 'Error al eliminar la presentación.');
        }
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  function openAddItemDialog() {
    closeActiveDialog();
    activeDialog = createAddItemDialog({
      onSubmit: async (payload) => {
        try {
          if (payload.type === 'track') {
            await uploadTrack(presentationId, {
              title: payload.title,
              file: payload.file,
            });
          } else {
            await addTextItem(presentationId, {
              type: payload.type,
              body: payload.body,
            });
          }
          closeActiveDialog();
          await loadData();
        } catch (err) {
          alert(err.message || 'Error al agregar el elemento.');
        }
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  async function handleMoveItem(item, direction) {
    try {
      const res = await moveItem(presentationId, item.id, direction);
      items = res.items || [];
      view.update({ items });
    } catch (err) {
      if (err.status === 409 || err.code === 'reorder_boundary') {
        // Boundary reached or concurrently modified
        return;
      }
      alert(err.message || 'Error al reordenar el elemento.');
      await loadData();
    }
  }

  function openDeleteItemDialog(item) {
    closeActiveDialog();
    const itemLabel =
      item.type === 'track'
        ? `la pista "${item.title}"`
        : item.type === 'lyrics'
        ? 'esta letra'
        : 'esta nota';

    activeDialog = createConfirmationDialog({
      title: '¿Eliminar elemento?',
      message: `¿Estás seguro de que deseas eliminar ${itemLabel} del show?`,
      confirmText: 'Eliminar',
      isDanger: true,
      onConfirm: async () => {
        try {
          const currentPlayback = playbackSignal.get();
          if (currentPlayback.currentTrackId === item.id) {
            audioController.stop();
          }

          const res = await deleteItem(presentationId, item.id);
          items = res.items || [];
          view.update({ items });
          closeActiveDialog();
        } catch (err) {
          alert(err.message || 'Error al eliminar el elemento.');
          await loadData();
        }
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  // Initial load
  loadData();

  function destroy() {
    unsubscribePlayback();
    audioController.stop();
    closeActiveDialog();
  }

  return {
    topbarEl: topbar.el,
    viewEl: view.el,
    destroy,
  };
}
