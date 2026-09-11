import { createShellLayout } from './components/layouts/shell-layout.js';
import { createAudioController } from './behaviors/create-audio-controller.js';
import { parseRoute } from './router/parse-route.js';
import { navigateTo } from './router/navigate.js';
import { activeRouteSignal } from './state/active-route.js';
import { createPresentationsController } from './controllers/presentations-controller.js';
import { createPresentationDetailController } from './controllers/presentation-detail-controller.js';

/**
 * Shirley Show Composition Root.
 * Initializes singleton audio controller, sets up shell layout, and handles routing.
 */
function init() {
  const rootEl = document.getElementById('app');
  if (!rootEl) {
    console.error('Root element #app not found.');
    return;
  }

  // Exactly ONE audio controller for the entire application session
  const audioController = createAudioController();

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
      });
      shell.setHeader(activeController.topbarEl);
      shell.setMain(activeController.viewEl);
    } else if (parsed.view === 'presentation-detail') {
      const presentationId = parsed.params.id;
      activeController = createPresentationDetailController({
        presentationId,
        audioController,
      });
      shell.setHeader(activeController.topbarEl);
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
