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
