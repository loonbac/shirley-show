import { createShellLayout } from './components/layouts/shell-layout.js';
import { createAudioController } from './behaviors/create-audio-controller.js';
import { parseRoute } from './router/parse-route.js';
import { navigateTo } from './router/navigate.js';
import { activeRouteSignal } from './state/active-route.js';
import { createPresentationsController } from './controllers/presentations-controller.js';
import { createPresentationDetailController } from './controllers/presentation-detail-controller.js';
import { createEventsCatalogController, createEventDetailController } from './modules/events/controllers.js?v=20260912-3';
import { createPublicTicketController } from './modules/events/public-ticket-controller.js';
import { createDoorController } from './modules/events/door-controller.js';
import { createChatState } from './core/state/chat-state.js';
import { createAgentChatController } from './controllers/agent-chat-controller.js';
import { bindAgentChatShortcuts } from './behaviors/bind-agent-chat-shortcuts.js';
import { stageModeSignal } from './core/state/stage-mode-state.js';

/**
 * Shirley Show Composition Root.
 * Initializes singleton audio controller, singleton AI agent chat controller,
 * sets up shell layout, and handles routing for presentations and ticketing.
 */
function init() {
  const rootEl = document.getElementById('app');
  if (!rootEl) {
    console.error('Root element #app not found.');
    return;
  }

  // Exactly ONE audio controller for the entire application session
  const audioController = createAudioController();

  // Exactly ONE in-memory chat state and agent controller for the entire session
  const chatState = createChatState();
  const agentChatController = createAgentChatController({ chatState });
  document.body.appendChild(agentChatController.panelEl);
  if (agentChatController.floatingTriggerEl) {
    document.body.appendChild(agentChatController.floatingTriggerEl);
  }

  // Global keyboard shortcuts (Ctrl+K / Cmd+K), suppressed during Stage Mode
  bindAgentChatShortcuts({
    onToggleChat: () => agentChatController.toggle(),
    isStageModeActive: () => stageModeSignal.get(),
  });

  // Root layout
  const shell = createShellLayout();
  rootEl.innerHTML = '';
  rootEl.appendChild(shell.el);

  let activeController = null;

  function route() {
    const parsed = parseRoute(window.location.hash);

    if (parsed.fallback) {
      navigateTo('#/presentations');
      return;
    }

    // Clean up previous controller
    if (activeController && typeof activeController.destroy === 'function') {
      activeController.destroy();
      activeController = null;
    }

    activeRouteSignal.set(parsed);

    if (parsed.view === 'presentations') {
      activeController = createPresentationsController({
        audioController,
        onOpenAgent: () => agentChatController.open(),
      });
      shell.setHeader(activeController.topbarEl);
      shell.setMain(activeController.viewEl);
    } else if (parsed.view === 'presentation-detail') {
      const presentationId = parsed.params.id;
      activeController = createPresentationDetailController({
        presentationId,
        audioController,
        onOpenAgent: () => agentChatController.open(),
      });
      shell.setHeader(activeController.topbarEl);
      shell.setMain(activeController.viewEl);
    } else if (parsed.view === 'events') {
      activeController = createEventsCatalogController({
        audioController,
        onOpenAgent: () => agentChatController.open(),
      });
      shell.setHeader(activeController.topbarEl);
      shell.setMain(activeController.viewEl);
    } else if (parsed.view === 'event-detail') {
      const eventId = parsed.params.id;
      activeController = createEventDetailController({
        eventId,
        audioController,
        onOpenAgent: () => agentChatController.open(),
      });
      shell.setHeader(activeController.topbarEl);
      shell.setMain(activeController.viewEl);
    } else if (parsed.view === 'public-ticket') {
      const scanToken = parsed.params.token;
      activeController = createPublicTicketController({
        scanToken,
        audioController,
      });
      shell.setHeader(activeController.headerEl);
      shell.setMain(activeController.viewEl);
    } else if (parsed.view === 'door') {
      const eventId = parsed.params.id;
      activeController = createDoorController({
        eventId,
        audioController,
      });
      shell.setHeader(activeController.headerEl || null);
      shell.setMain(activeController.viewEl);
    }
  }

  // Listen for hash changes
  window.addEventListener('hashchange', route);

  // Initial route
  if (!window.location.hash || window.location.hash === '#') {
    window.location.hash = '#/presentations';
  } else {
    route();
  }
}

// Start application when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}
