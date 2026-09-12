import { createAppTopbar } from '../../components/organisms/app-topbar.js';
import { createAgentChatTrigger } from '../../components/molecules/agent-chat-trigger.js';
import { createConfirmationDialog } from '../../components/molecules/confirmation-dialog.js';
import { getActiveAgentChatController } from '../../controllers/agent-chat-controller.js';
import { activePresentationContextSignal } from '../../core/state/active-presentation-context.js';
import { toPresentations, toEvent, toEvents, toDoor } from '../../router/navigate.js';
import { buildWhatsAppTicketLink } from './whatsapp.js';

import {
  listEvents,
  createEvent,
  getEvent,
  updateEvent,
  deleteEvent,
  finalizeEvent,
  reopenEvent,
  uploadEventPoster,
  getEventSummary,
  listTickets,
  sellTickets,
  confirmTicketPayment,
  deleteTicket,
  listInterests,
  createInterest,
  updateInterest,
  deleteInterest,
  convertInterestToSale,
  listExpenses,
  createExpense,
  updateExpense,
  deleteExpense,
} from './api.js';

import { createEventsCatalogView, createEventDetailView } from './views.js';
import {
  createEventFormDialog,
  createExpenseFormDialog,
  createInterestFormDialog,
  createConvertInterestDialog,
  createSellTicketDialog,
} from './forms.js';

/**
 * Controller orchestrating the Events Catalog (#/eventos).
 * Handles loading list of events, create/edit/delete modals, and navigation to details.
 *
 * @param {{
 *   audioController?: { stop: () => void },
 *   onOpenAgent?: (initialMessage?: string) => void,
 * }} deps 
 * @returns {{
 *   topbarEl: HTMLElement,
 *   viewEl: HTMLElement,
 *   destroy: () => void,
 * }}
 */
export function createEventsCatalogController(deps = {}) {
  const { audioController, onOpenAgent } = deps;

  if (audioController && typeof audioController.stop === 'function') {
    audioController.stop();
  }

  activePresentationContextSignal.set(null);

  let events = [];
  let isLoading = true;
  let errorMessage = null;
  let activeDialog = null;

  function closeActiveDialog() {
    if (activeDialog) {
      activeDialog.destroy();
      activeDialog = null;
    }
  }

  // Agent chat trigger in topbar
  const agentChatTrigger = createAgentChatTrigger({
    onClick: () => {
      const agentController = getActiveAgentChatController();
      if (agentController && typeof agentController.toggle === 'function') {
        agentController.toggle();
      } else if (typeof onOpenAgent === 'function') {
        onOpenAgent();
      }
    },
  });

  const agentController = getActiveAgentChatController();
  let unsubscribeChat = null;
  if (agentController && typeof agentController.subscribe === 'function') {
    unsubscribeChat = agentController.subscribe((chatState) => {
      agentChatTrigger.update({ isOpen: chatState.isOpen });
    });
  }

  const topbar = createAppTopbar({
    title: 'Eventos',
    activeSection: 'events',
    onSectionChange: (sec) => {
      if (sec === 'presentations') toPresentations();
    },
    actions: [agentChatTrigger.el],
  });

  async function loadData() {
    isLoading = true;
    errorMessage = null;
    view.update({ isLoading: true, errorMessage: null });

    try {
      events = await listEvents({ incluir_finalizados: true });
      isLoading = false;
      view.update({ events, isLoading: false, errorMessage: null });
    } catch (err) {
      isLoading = false;
      errorMessage = (err && err.message) || 'Error al cargar los eventos.';
      view.update({ isLoading: false, errorMessage });
    }
  }

  function handleCreateEvent() {
    closeActiveDialog();
    activeDialog = createEventFormDialog({
      onSave: async (data) => {
        await createEvent(data);
        closeActiveDialog();
        await loadData();
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  function handleEditEvent(event) {
    closeActiveDialog();
    activeDialog = createEventFormDialog({
      event,
      onSave: async (data) => {
        await updateEvent(event.id, data);
        closeActiveDialog();
        await loadData();
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  function handleDeleteEvent(event) {
    closeActiveDialog();
    activeDialog = createConfirmationDialog({
      title: '¿Eliminar Evento?',
      message: `¿Estás seguro de eliminar "${event.nombre}"? Esta acción borrará todas las entradas, interesados y gastos asociados de forma irreversible.`,
      confirmText: 'Eliminar Evento',
      isDanger: true,
      onConfirm: async () => {
        await deleteEvent(event.id);
        closeActiveDialog();
        await loadData();
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  const view = createEventsCatalogView({
    events,
    isLoading,
    errorMessage,
    onCreateEvent: handleCreateEvent,
    onSelectEvent: (id) => toEvent(id),
    onEditEvent: handleEditEvent,
    onDeleteEvent: handleDeleteEvent,
  });

  loadData();

  function destroy() {
    closeActiveDialog();
    if (typeof unsubscribeChat === 'function') {
      unsubscribeChat();
      unsubscribeChat = null;
    }
  }

  return {
    topbarEl: topbar.el,
    viewEl: view.el,
    destroy,
  };
}

/**
 * Controller orchestrating the Event Detail View (#/eventos/:id).
 * Manages event lifecycle, tabs (Resumen, Entradas, Interesados, Gastos),
 * modal dialogs, and updates the agent chat context.
 *
 * @param {{
 *   eventId: string,
 *   audioController?: { stop: () => void },
 *   onOpenAgent?: (initialMessage?: string) => void,
 * }} deps 
 * @returns {{
 *   topbarEl: HTMLElement,
 *   viewEl: HTMLElement,
 *   destroy: () => void,
 * }}
 */
export function createEventDetailController(deps) {
  const { eventId, audioController, onOpenAgent } = deps;

  if (audioController && typeof audioController.stop === 'function') {
    audioController.stop();
  }

  let event = null;
  let summary = null;
  let tickets = [];
  let interests = [];
  let expenses = [];
  let activeTab = 'resumen';
  let activeDialog = null;

  function closeActiveDialog() {
    if (activeDialog) {
      activeDialog.destroy();
      activeDialog = null;
    }
  }

  // Agent chat trigger in topbar
  const agentChatTrigger = createAgentChatTrigger({
    onClick: () => {
      const agentController = getActiveAgentChatController();
      if (agentController && typeof agentController.toggle === 'function') {
        agentController.toggle();
      } else if (typeof onOpenAgent === 'function') {
        onOpenAgent();
      }
    },
  });

  const agentController = getActiveAgentChatController();
  let unsubscribeChat = null;
  if (agentController && typeof agentController.subscribe === 'function') {
    unsubscribeChat = agentController.subscribe((chatState) => {
      agentChatTrigger.update({ isOpen: chatState.isOpen });
    });
  }

  const topbar = createAppTopbar({
    title: 'Cargando evento...',
    onBack: () => toEvents(),
    activeSection: 'events',
    onSectionChange: (sec) => {
      if (sec === 'presentations') toPresentations();
    },
    actions: [agentChatTrigger.el],
  });

  async function loadAllData() {
    try {
      const [evtData, sumData, tixData, intData, expData] = await Promise.all([
        getEvent(eventId),
        getEventSummary(eventId),
        listTickets(eventId),
        listInterests(eventId),
        listExpenses(eventId),
      ]);

      event = evtData;
      summary = sumData;
      tickets = tixData || [];
      interests = intData || [];
      expenses = expData || [];

      topbar.update({ title: event.nombre });
      activePresentationContextSignal.set({
        id: event.id,
        title: event.nombre,
        type: 'event',
      });

      view.update({
        event,
        summary,
        tickets,
        interests,
        expenses,
        activeTab,
      });
    } catch (err) {
      console.error('Error loading event detail:', err);
      topbar.update({ title: 'Error al cargar evento' });
    }
  }

  // Event actions
  async function handleFinalize() {
    try {
      await finalizeEvent(eventId);
      await loadAllData();
    } catch (err) {
      alert((err && err.message) || 'Error al finalizar evento.');
    }
  }

  async function handleReopen() {
    try {
      await reopenEvent(eventId);
      await loadAllData();
    } catch (err) {
      alert((err && err.message) || 'Error al reabrir evento.');
    }
  }

  async function handleUploadPoster(file) {
    try {
      await uploadEventPoster(eventId, file);
      await loadAllData();
    } catch (err) {
      alert((err && err.message) || 'Error al subir afiche.');
    }
  }

  function handleEditEvent() {
    closeActiveDialog();
    activeDialog = createEventFormDialog({
      event,
      onSave: async (data) => {
        await updateEvent(eventId, data);
        closeActiveDialog();
        await loadAllData();
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  function handleDeleteEvent() {
    closeActiveDialog();
    activeDialog = createConfirmationDialog({
      title: '¿Eliminar Evento?',
      message: `¿Deseas eliminar permanentemente "${event ? event.nombre : ''}"? Se perderán todas las entradas y gastos.`,
      confirmText: 'Eliminar Evento',
      isDanger: true,
      onConfirm: async () => {
        await deleteEvent(eventId);
        closeActiveDialog();
        toEvents();
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  // Tickets actions
  function handleTicketWhatsApp(ticket) {
    const link = buildWhatsAppTicketLink({
      ticket,
      eventName: (event && event.nombre) || '',
    });
    if (typeof window !== 'undefined' && window.open) {
      window.open(link, '_blank', 'noopener,noreferrer');
    }
  }

  function handleSellTickets() {
    closeActiveDialog();
    activeDialog = createSellTicketDialog({
      eventName: (event && event.nombre) || '',
      ticketTypes: (event && event.tipos_de_entrada) || [],
      onSell: async (data) => {
        const issued = await sellTickets(eventId, data);
        await loadAllData();
        return issued;
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  async function handleConfirmPayment(ticketId) {
    try {
      await confirmTicketPayment(ticketId);
      await loadAllData();
    } catch (err) {
      alert((err && err.message) || 'Error al confirmar pago.');
    }
  }

  function handleDeleteTicket(ticket) {
    closeActiveDialog();
    activeDialog = createConfirmationDialog({
      title: '¿Eliminar Entrada?',
      message: `¿Eliminar la entrada de ${ticket.nombre_comprador}?`,
      confirmText: 'Eliminar',
      isDanger: true,
      onConfirm: async () => {
        await deleteTicket(ticket.id);
        closeActiveDialog();
        await loadAllData();
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  // Interests actions
  function handleCreateInterest() {
    closeActiveDialog();
    activeDialog = createInterestFormDialog({
      onSave: async (data) => {
        await createInterest(eventId, data);
        closeActiveDialog();
        await loadAllData();
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  function handleEditInterest(interest) {
    closeActiveDialog();
    activeDialog = createInterestFormDialog({
      interest,
      onSave: async (data) => {
        await updateInterest(interest.id, data);
        closeActiveDialog();
        await loadAllData();
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  function handleDeleteInterest(interest) {
    closeActiveDialog();
    activeDialog = createConfirmationDialog({
      title: '¿Eliminar Interesado?',
      message: `¿Eliminar a ${interest.nombre} de la lista de interesados?`,
      confirmText: 'Eliminar',
      isDanger: true,
      onConfirm: async () => {
        await deleteInterest(interest.id);
        closeActiveDialog();
        await loadAllData();
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  function handleConvertInterest(interest) {
    closeActiveDialog();
    activeDialog = createConvertInterestDialog({
      interest,
      ticketTypes: (event && event.tipos_de_entrada) || [],
      onConvert: async (data) => {
        await convertInterestToSale(interest.id, data);
        closeActiveDialog();
        await loadAllData();
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  // Expenses actions
  function handleCreateExpense() {
    closeActiveDialog();
    activeDialog = createExpenseFormDialog({
      onSave: async (data) => {
        await createExpense(eventId, data);
        closeActiveDialog();
        await loadAllData();
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  function handleEditExpense(expense) {
    closeActiveDialog();
    activeDialog = createExpenseFormDialog({
      expense,
      onSave: async (data) => {
        await updateExpense(expense.id, data);
        closeActiveDialog();
        await loadAllData();
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  function handleDeleteExpense(expense) {
    closeActiveDialog();
    activeDialog = createConfirmationDialog({
      title: '¿Eliminar Gasto?',
      message: `¿Eliminar el gasto "${expense.descripcion}" por S/ ${Number(expense.monto).toFixed(2)}?`,
      confirmText: 'Eliminar',
      isDanger: true,
      onConfirm: async () => {
        await deleteExpense(expense.id);
        closeActiveDialog();
        await loadAllData();
      },
      onCancel: closeActiveDialog,
    });
    document.body.appendChild(activeDialog.el);
  }

  const view = createEventDetailView({
    event,
    summary,
    tickets,
    interests,
    expenses,
    activeTab,
    onTabChange: (tab) => {
      activeTab = tab;
      view.update({ activeTab });
    },
    onFinalize: handleFinalize,
    onReopen: handleReopen,
    onUploadPoster: handleUploadPoster,
    onOpenDoor: () => toDoor(eventId),
    onEditEvent: handleEditEvent,
    onDeleteEvent: handleDeleteEvent,
    onSellTickets: handleSellTickets,
    onConfirmPayment: handleConfirmPayment,
    onTicketWhatsApp: handleTicketWhatsApp,
    onDeleteTicket: handleDeleteTicket,
    onCreateInterest: handleCreateInterest,
    onEditInterest: handleEditInterest,
    onDeleteInterest: handleDeleteInterest,
    onConvertInterest: handleConvertInterest,
    onCreateExpense: handleCreateExpense,
    onEditExpense: handleEditExpense,
    onDeleteExpense: handleDeleteExpense,
  });

  loadAllData();

  function destroy() {
    closeActiveDialog();
    if (typeof unsubscribeChat === 'function') {
      unsubscribeChat();
      unsubscribeChat = null;
    }
  }

  return {
    topbarEl: topbar.el,
    viewEl: view.el,
    destroy,
  };
}
