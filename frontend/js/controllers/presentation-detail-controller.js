import { createAppTopbar } from '../components/organisms/app-topbar.js';
import { createButton } from '../components/atoms/button.js';
import { createIcon } from '../components/atoms/icon.js';
import { createPresentationDetailView } from '../components/views/presentation-detail-view.js';
import { createPresentationFormDialog } from '../components/organisms/presentation-form-dialog.js';
import { createAddItemDialog } from '../components/organisms/add-item-dialog.js';
import { createQuickEditDialog } from '../components/organisms/quick-edit-dialog.js';
import { createConfirmationDialog } from '../components/molecules/confirmation-dialog.js';
import { createStageModeView } from '../components/organisms/stage-mode-view.js';
import { useWakeLock } from '../behaviors/use-wake-lock.js';
import { activePresentationContextSignal } from '../core/state/active-presentation-context.js';
import { setStageModeActive } from '../core/state/stage-mode-state.js';
import { onAgentRefresh } from '../core/state/agent-refresh.js';
import { getPresentation } from '../data/get-presentation.js';
import { renamePresentation } from '../data/rename-presentation.js';
import { deletePresentation } from '../data/delete-presentation.js';
import { addTextItem } from '../data/add-text-item.js';
import { uploadTrack } from '../data/upload-track.js';
import { updateItem } from '../data/update-item.js';
import { moveItem } from '../data/move-item.js';
import { deleteItem } from '../data/delete-item.js';
import { toPresentations, toEvents } from '../router/navigate.js';
import { playbackSignal } from '../state/playback-state.js';

/**
 * Controller orchestrating a presentation's vertical timeline, quick actions,
 * headless drag-and-drop reordering, distraction-free Stage Mode, and exclusive audio playback.
 *
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
  let confirmedItems = [];
  let isLoading = true;
  let errorMessage = null;
  let activeDialog = null;
  let stageModeView = null;
  let isReordering = false;

  function closeActiveDialog() {
    if (activeDialog) {
      activeDialog.destroy();
      activeDialog = null;
    }
  }

  // Screen Wake Lock behavior
  const wakeLock = useWakeLock({
    onChange: ({ status }) => {
      if (stageModeView) {
        stageModeView.update({ wakeLockStatus: status });
      }
    },
  });

  function enterStageMode() {
    if (stageModeView || !presentation) return;

    setStageModeActive(true);

    stageModeView = createStageModeView({
      presentation,
      items,
      playbackState: playbackSignal.get(),
      wakeLockStatus: wakeLock.getStatus(),
      onExit: exitStageMode,
      onTogglePlay: (track) => {
        audioController.toggle(track);
      },
      onSeek: (seconds) => {
        audioController.seek(seconds);
      },
    });

    document.body.appendChild(stageModeView.el);

    // Progressive Fullscreen: graceful degradation if unsupported or denied
    try {
      if (stageModeView.el.requestFullscreen) {
        stageModeView.el.requestFullscreen().catch(() => {});
      } else if (document.documentElement.requestFullscreen) {
        document.documentElement.requestFullscreen().catch(() => {});
      }
    } catch {
      // ignore
    }

    // Progressive Screen Wake Lock
    wakeLock.activate();
  }

  function exitStageMode() {
    if (!stageModeView) return;

    setStageModeActive(false);
    wakeLock.deactivate();

    try {
      if (document.fullscreenElement) {
        document.exitFullscreen().catch(() => {});
      }
    } catch {
      // ignore
    }

    stageModeView.destroy();
    stageModeView = null;
  }

  const stageBtn = createButton({
    text: 'Modo Escenario',
    variant: 'secondary',
    icon: createIcon({ name: 'play', size: 'sm' }),
    onClick: () => enterStageMode(),
  });

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
    activeSection: 'presentations',
    onSectionChange: (sec) => {
      if (sec === 'events') {
        audioController.stop();
        toEvents();
      }
    },
    actions: [stageBtn.el, addItemBtn.el],
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
    onEnterStageMode: () => enterStageMode(),
    onQuickAdd: (type) => openAddItemDialog(type),
    onEditItem: (item) => openQuickEditDialog(item),
    onReorderItem: (itemId, targetPosition) => handleDragReorder(itemId, targetPosition),
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
    if (stageModeView) {
      stageModeView.update({ playbackState: newPlaybackState });
    }
  });

  // Subscribe to agent refresh events for this presentation
  const unsubscribeRefresh = onAgentRefresh(async (event) => {
    if (event.scope === 'presentation' && event.presentationId === presentationId) {
      const scrollY =
        (typeof window !== 'undefined' && window.scrollY) ||
        (typeof document !== 'undefined' && document.documentElement && document.documentElement.scrollTop) ||
        (typeof document !== 'undefined' && document.body && document.body.scrollTop) ||
        0;
      const scrollEl = view.el?.parentElement || view.el;
      const elemScrollTop = scrollEl ? scrollEl.scrollTop : 0;

      await loadData();

      if (typeof window !== 'undefined' && typeof window.scrollTo === 'function') {
        window.scrollTo(0, scrollY);
      }
      if (typeof document !== 'undefined') {
        if (document.documentElement && document.documentElement.scrollTop !== undefined) {
          document.documentElement.scrollTop = scrollY;
        }
        if (document.body && document.body.scrollTop !== undefined) {
          document.body.scrollTop = scrollY;
        }
      }
      if (scrollEl && scrollEl.scrollTop !== undefined) {
        scrollEl.scrollTop = elemScrollTop;
      }
    }
  });

  async function loadData() {
    isLoading = true;
    errorMessage = null;
    view.update({ isLoading: true, errorMessage: null });

    try {
      const data = await getPresentation(presentationId);
      presentation = data.presentation;
      items = data.items || [];
      confirmedItems = [...items];
      isLoading = false;

      // Publish active presentation context for the AI agent
      activePresentationContextSignal.set({
        id: presentation.id,
        title: presentation.title,
      });

      topbar.update({ title: presentation.title });
      view.update({ presentation, items, isLoading: false });
      if (stageModeView) {
        stageModeView.update({ presentation, items });
      }
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
          activePresentationContextSignal.set({
            id: presentation.id,
            title: presentation.title,
          });
          topbar.update({ title: presentation.title });
          view.update({ presentation });
          if (stageModeView) {
            stageModeView.update({ presentation });
          }
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

  function openAddItemDialog(initialType) {
    closeActiveDialog();
    activeDialog = createAddItemDialog({
      initialType,
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

  function openQuickEditDialog(item) {
    closeActiveDialog();
    activeDialog = createQuickEditDialog({
      item,
      onSave: async (payload) => {
        try {
          const updatedItem = await updateItem(presentationId, item.id, payload);
          // Update item in local items array and refresh view
          items = items.map((it) => (it.id === updatedItem.id ? updatedItem : it));
          confirmedItems = [...items];
          view.update({ items });
          if (stageModeView) {
            stageModeView.update({ items });
          }
          closeActiveDialog();
        } catch (err) {
          if (err && (err.status === 404 || err.code === 'item_not_found')) {
            throw new Error('Este elemento ya no existe en el servidor. Puede haber sido eliminado.');
          }
          throw err;
        }
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  async function handleDragReorder(itemId, targetPosition) {
    if (isReordering) return;
    isReordering = true;

    try {
      const res = await moveItem(presentationId, itemId, { targetPosition });
      items = res.items || [];
      confirmedItems = [...items];
      view.update({ items });
      if (stageModeView) {
        stageModeView.update({ items });
      }
    } catch (err) {
      if (err.status === 409 || err.code === 'reorder_boundary') {
        // Boundary reached or conflict: restore confirmed order
        items = [...confirmedItems];
        view.update({ items });
        return;
      }
      alert(err.message || 'Error al reordenar el elemento.');
      items = [...confirmedItems];
      view.update({ items });
      await loadData();
    } finally {
      isReordering = false;
    }
  }

  async function handleMoveItem(item, direction) {
    try {
      const res = await moveItem(presentationId, item.id, direction);
      items = res.items || [];
      confirmedItems = [...items];
      view.update({ items });
      if (stageModeView) {
        stageModeView.update({ items });
      }
    } catch (err) {
      if (err.status === 409 || err.code === 'reorder_boundary') {
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
          confirmedItems = [...items];
          view.update({ items });
          if (stageModeView) {
            stageModeView.update({ items });
          }
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
    setStageModeActive(false);
    unsubscribeRefresh();
    unsubscribePlayback();
    exitStageMode();
    wakeLock.destroy();
    audioController.stop();
    closeActiveDialog();
    if (view.destroy) {
      view.destroy();
    }
  }

  return {
    topbarEl: topbar.el,
    viewEl: view.el,
    destroy,
  };
}
