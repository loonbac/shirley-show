import { createElement } from '../utils/dom/create-element.js';
import { createAppTopbar } from '../components/organisms/app-topbar.js';
import { createPresentationsView } from '../components/views/presentations-view.js';
import { createPresentationFormDialog } from '../components/organisms/presentation-form-dialog.js';
import { createConfirmationDialog } from '../components/molecules/confirmation-dialog.js';
import { getActiveAgentChatController } from './agent-chat-controller.js';
import { activePresentationContextSignal } from '../core/state/active-presentation-context.js';
import { onAgentRefresh } from '../core/state/agent-refresh.js';
import { listPresentations } from '../data/list-presentations.js';
import { createPresentation } from '../data/create-presentation.js';
import { renamePresentation } from '../data/rename-presentation.js';
import { deletePresentation } from '../data/delete-presentation.js';
import { toPresentation, toEvents } from '../router/navigate.js';

/**
 * Controller orchestrating the presentations catalog.
 * Consumes enriched presentation summaries, calculates aggregate library metrics,
 * handles presentation lifecycle actions (open, create, rename, delete),
 * and routes messages from the Ressy Hero chat directly to the AI agent controller.
 *
 * @param {{
 *   audioController: { stop: () => void },
 * }} deps 
 * @returns {{
 *   topbarEl: HTMLElement,
 *   viewEl: HTMLElement,
 *   destroy: () => void,
 * }}
 */
export function createPresentationsController(deps) {
  const { audioController } = deps;

  // Enforce exclusive playback: stop audio when on catalog view
  if (audioController) {
    audioController.stop();
  }

  // Catalog view has no active presentation context
  activePresentationContextSignal.set(null);

  let presentations = [];
  let isLoading = true;
  let errorMessage = null;
  let activeDialog = null;

  function closeActiveDialog() {
    if (activeDialog) {
      activeDialog.destroy();
      activeDialog = null;
    }
  }

  function handleHeroSend(message) {
    const agentController = getActiveAgentChatController();
    if (agentController && typeof agentController.sendAndOpen === 'function') {
      agentController.sendAndOpen(message);
    }
  }

  const topbar = createAppTopbar({
    title: 'Shirley Show',
    activeSection: 'presentations',
    onSectionChange: (sec) => {
      if (sec === 'events') {
        toEvents();
      }
    },
    actions: [],
  });

  const view = createPresentationsView({
    presentations,
    totalPresentations: 0,
    totalTracks: 0,
    isLoading,
    errorMessage,
    onCreateClick: () => openCreateDialog(),
    onSelect: (id) => toPresentation(id),
    onRename: (id, currentTitle) => openRenameDialog(id, currentTitle),
    onDelete: (id, title) => openDeleteDialog(id, title),
    onSendMessage: (message) => handleHeroSend(message),
  });

  async function loadData() {
    isLoading = true;
    errorMessage = null;
    view.update({ isLoading: true, errorMessage: null });

    try {
      const data = await listPresentations();
      presentations = (data && data.presentations) || [];

      const totalPresentations = presentations.length;
      const totalTracks = presentations.reduce(
        (acc, pres) => acc + (Number(pres.tracks_count) || 0),
        0
      );

      isLoading = false;
      view.update({
        presentations,
        totalPresentations,
        totalTracks,
        isLoading: false,
        errorMessage: null,
      });
    } catch (err) {
      isLoading = false;
      errorMessage = err.message || 'Error al cargar las presentaciones.';
      view.update({ isLoading: false, errorMessage });
    }
  }

  function openCreateDialog() {
    closeActiveDialog();
    activeDialog = createPresentationFormDialog({
      mode: 'create',
      onSubmit: async (title) => {
        try {
          await createPresentation({ title });
          closeActiveDialog();
          await loadData();
        } catch (err) {
          alert(err.message || 'Error al crear la presentación.');
        }
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  function openRenameDialog(id, currentTitle) {
    closeActiveDialog();
    activeDialog = createPresentationFormDialog({
      mode: 'rename',
      initialTitle: currentTitle,
      onSubmit: async (newTitle) => {
        try {
          await renamePresentation(id, { title: newTitle });
          closeActiveDialog();
          await loadData();
        } catch (err) {
          alert(err.message || 'Error al renombrar la presentación.');
        }
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  function openDeleteDialog(id, title) {
    closeActiveDialog();
    activeDialog = createConfirmationDialog({
      title: '¿Eliminar presentación?',
      message: `¿Estás seguro de que deseas eliminar "${title}"? Se borrarán permanentemente todos sus elementos y pistas de audio asociadas.`,
      confirmText: 'Eliminar',
      isDanger: true,
      onConfirm: async () => {
        try {
          await deletePresentation(id);
          closeActiveDialog();
          await loadData();
        } catch (err) {
          alert(err.message || 'Error al eliminar la presentación.');
        }
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  // Subscribe to reactive agent refresh events for catalog scope
  const unsubscribeRefresh = onAgentRefresh((event) => {
    if (event.scope === 'catalog') {
      loadData();
    }
  });

  // Initial load
  loadData();

  function destroy() {
    unsubscribeRefresh();
    closeActiveDialog();
  }

  return {
    topbarEl: topbar.el,
    viewEl: view.el,
    destroy,
  };
}
