import { createElement } from '../utils/dom/create-element.js';
import { createAppTopbar } from '../components/organisms/app-topbar.js';
import { createButton } from '../components/atoms/button.js';
import { createIcon } from '../components/atoms/icon.js';
import { createPresentationsView } from '../components/views/presentations-view.js';
import { createPresentationFormDialog } from '../components/organisms/presentation-form-dialog.js';
import { createConfirmationDialog } from '../components/molecules/confirmation-dialog.js';
import { listPresentations } from '../data/list-presentations.js';
import { createPresentation } from '../data/create-presentation.js';
import { renamePresentation } from '../data/rename-presentation.js';
import { deletePresentation } from '../data/delete-presentation.js';
import { toPresentation } from '../router/navigate.js';

/**
 * Controller orchestrating the presentations catalog.
 * Consumes enriched presentation summaries, calculates aggregate library metrics,
 * and handles presentation lifecycle actions (open, create, rename, delete).
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

  const newPresBtn = createButton({
    text: 'Nueva Presentación',
    variant: 'primary',
    icon: createIcon({ name: 'plus', size: 'sm' }),
    onClick: () => openCreateDialog(),
  });

  const topbar = createAppTopbar({
    title: 'Shirley Show',
    actions: [newPresBtn.el],
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

  // Initial load
  loadData();

  function destroy() {
    closeActiveDialog();
  }

  return {
    topbarEl: topbar.el,
    viewEl: view.el,
    destroy,
  };
}
