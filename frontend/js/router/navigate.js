/**
 * Programmatically changes the URL hash.
 * @param {string} hash 
 */
export function navigateTo(hash) {
  const target = hash.startsWith('#') ? hash : `#${hash}`;
  if (window.location.hash !== target) {
    window.location.hash = target;
  }
}

/**
 * Navigates to the presentations catalog view.
 */
export function toPresentations() {
  navigateTo('#/presentations');
}

/**
 * Navigates to a specific presentation detail view.
 * @param {string} id 
 */
export function toPresentation(id) {
  navigateTo(`#/presentations/${encodeURIComponent(id)}`);
}

/**
 * Navigates to the events catalog view.
 */
export function toEvents() {
  navigateTo('#/eventos');
}

/**
 * Navigates to a specific event detail view.
 * @param {string} id 
 */
export function toEvent(id) {
  navigateTo(`#/eventos/${encodeURIComponent(id)}`);
}

/**
 * Navigates to the public ticket view for a given scan_token.
 * @param {string} token 
 */
export function toPublicTicket(token) {
  navigateTo(`#/ticket/${encodeURIComponent(token)}`);
}

/**
 * Navigates to the door / check-in view for a given event id.
 * @param {string} id 
 */
export function toDoor(id) {
  navigateTo(`#/puerta/${encodeURIComponent(id)}`);
}
