/**
 * Headless drag-and-drop reordering behavior for sequence lists.
 * Captures pointer events strictly on dedicated drag handles with touch-action: none.
 * Calculates insertion thresholds via requestAnimationFrame, applies temporary visual
 * feedback without altering the real DOM, emits onReorder(itemId, targetPosition) upon release,
 * and restores state upon pointercancel or out-of-bounds release.
 *
 * @param {{
 *   listEl: HTMLElement,
 *   handleSelector?: string,
 *   itemSelector?: string,
 *   onReorder?: (itemId: string, targetPosition: number) => void,
 *   onCancel?: () => void,
 * }} options 
 * @returns {{
 *   destroy: () => void,
 * }}
 */
export function useDragAndDrop(options) {
  const {
    listEl,
    handleSelector = '.drag-handle',
    itemSelector = '[data-id]',
    onReorder,
    onCancel,
  } = options;

  if (!listEl) {
    return { destroy: () => {} };
  }

  let isDragging = false;
  let activePointerId = null;
  let activeHandleEl = null;
  let activeItemEl = null;
  let activeItemId = null;
  let startIndex = -1;
  let currentTargetIndex = -1;
  let items = [];
  let rafId = null;
  let lastClientX = 0;
  let lastClientY = 0;

  function clearVisualFeedback() {
    if (activeItemEl) {
      activeItemEl.classList.remove('is-dragging');
    }
    if (activeHandleEl) {
      activeHandleEl.classList.remove('is-dragging');
    }
    listEl.classList.remove('is-reordering');

    for (const it of items) {
      it.classList.remove('drop-target-before', 'drop-target-after');
    }
  }

  function calculateThresholds() {
    if (!isDragging || items.length === 0) return;

    const listRect = listEl.getBoundingClientRect();
    const isInsideX = lastClientX >= listRect.left - 60 && lastClientX <= listRect.right + 60;
    const isInsideY = lastClientY >= listRect.top - 40 && lastClientY <= listRect.bottom + 40;

    if (!isInsideX || !isInsideY) {
      // Pointer is currently outside the list bounds
      for (const it of items) {
        it.classList.remove('drop-target-before', 'drop-target-after');
      }
      currentTargetIndex = startIndex;
      return;
    }

    let targetIdx = startIndex;
    for (let i = 0; i < items.length; i++) {
      const rect = items[i].getBoundingClientRect();
      const midY = rect.top + rect.height / 2;
      if (lastClientY < midY) {
        targetIdx = i;
        break;
      } else {
        targetIdx = i;
      }
    }

    targetIdx = Math.max(0, Math.min(items.length - 1, targetIdx));
    currentTargetIndex = targetIdx;

    // Apply visual indicator without moving real DOM
    for (let i = 0; i < items.length; i++) {
      items[i].classList.remove('drop-target-before', 'drop-target-after');
      if (i === currentTargetIndex && currentTargetIndex !== startIndex) {
        if (currentTargetIndex < startIndex) {
          items[i].classList.add('drop-target-before');
        } else {
          items[i].classList.add('drop-target-after');
        }
      }
    }
  }

  function onPointerDown(e) {
    // Only respond to primary mouse button or touch/pen
    if (e.button !== undefined && e.button !== 0) return;

    const handle = e.target.closest(handleSelector);
    if (!handle || handle.disabled) return;

    // Verify handle belongs to listEl
    if (!listEl.contains(handle)) return;

    const itemEl = handle.closest(itemSelector);
    if (!itemEl || !listEl.contains(itemEl)) return;

    isDragging = true;
    activePointerId = e.pointerId;
    activeHandleEl = handle;
    activeItemEl = itemEl;
    activeItemId = itemEl.dataset.id;
    lastClientX = e.clientX;
    lastClientY = e.clientY;

    items = Array.from(listEl.querySelectorAll(itemSelector));
    startIndex = items.indexOf(itemEl);
    currentTargetIndex = startIndex;

    try {
      handle.setPointerCapture(e.pointerId);
    } catch {
      // ignore
    }

    listEl.classList.add('is-reordering');
    activeItemEl.classList.add('is-dragging');
    activeHandleEl.classList.add('is-dragging');

    window.addEventListener('pointermove', onPointerMove, { passive: true });
    window.addEventListener('pointerup', onPointerUp);
    window.addEventListener('pointercancel', onPointerCancel);
  }

  function onPointerMove(e) {
    if (!isDragging || e.pointerId !== activePointerId) return;

    lastClientX = e.clientX;
    lastClientY = e.clientY;

    if (rafId === null) {
      rafId = requestAnimationFrame(() => {
        rafId = null;
        calculateThresholds();
      });
    }
  }

  function onPointerUp(e) {
    if (!isDragging || e.pointerId !== activePointerId) return;

    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }

    try {
      if (activeHandleEl && activeHandleEl.hasPointerCapture(activePointerId)) {
        activeHandleEl.releasePointerCapture(activePointerId);
      }
    } catch {
      // ignore
    }

    window.removeEventListener('pointermove', onPointerMove);
    window.removeEventListener('pointerup', onPointerUp);
    window.removeEventListener('pointercancel', onPointerCancel);

    const listRect = listEl.getBoundingClientRect();
    const isInside =
      e.clientY >= listRect.top - 40 &&
      e.clientY <= listRect.bottom + 40 &&
      e.clientX >= listRect.left - 60 &&
      e.clientX <= listRect.right + 60;

    const targetPos = currentTargetIndex;
    const fromId = activeItemId;
    const hasMoved = targetPos !== startIndex && targetPos >= 0 && targetPos < items.length;

    clearVisualFeedback();

    isDragging = false;
    activePointerId = null;
    activeHandleEl = null;
    activeItemEl = null;
    activeItemId = null;
    startIndex = -1;
    currentTargetIndex = -1;
    items = [];

    if (isInside && hasMoved && fromId && typeof onReorder === 'function') {
      onReorder(fromId, targetPos);
    } else if (typeof onCancel === 'function') {
      onCancel();
    }
  }

  function onPointerCancel(e) {
    if (!isDragging || (e && e.pointerId !== activePointerId)) return;

    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }

    try {
      if (activeHandleEl && activeHandleEl.hasPointerCapture(activePointerId)) {
        activeHandleEl.releasePointerCapture(activePointerId);
      }
    } catch {
      // ignore
    }

    window.removeEventListener('pointermove', onPointerMove);
    window.removeEventListener('pointerup', onPointerUp);
    window.removeEventListener('pointercancel', onPointerCancel);

    clearVisualFeedback();

    isDragging = false;
    activePointerId = null;
    activeHandleEl = null;
    activeItemEl = null;
    activeItemId = null;
    startIndex = -1;
    currentTargetIndex = -1;
    items = [];

    if (typeof onCancel === 'function') {
      onCancel();
    }
  }

  listEl.addEventListener('pointerdown', onPointerDown);

  function destroy() {
    listEl.removeEventListener('pointerdown', onPointerDown);
    window.removeEventListener('pointermove', onPointerMove);
    window.removeEventListener('pointerup', onPointerUp);
    window.removeEventListener('pointercancel', onPointerCancel);

    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }

    if (isDragging) {
      clearVisualFeedback();
      isDragging = false;
    }
  }

  return {
    destroy,
  };
}
