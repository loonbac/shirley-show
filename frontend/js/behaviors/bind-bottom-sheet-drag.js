/**
 * Headless interactive drag behavior for bottom sheets.
 * Follows finger movement 1:1 in real time during touch gestures, with physics-based
 * resistance at boundaries and velocity-aware spring snapping upon release.
 *
 * @param {Object} options
 * @param {HTMLElement} options.sheetEl - The bottom sheet element
 * @param {Array<HTMLElement>} [options.dragHandles=[]] - Handle bars or headers that initiate sheet drag
 * @param {HTMLElement} [options.scrollEl=null] - Scrollable container inside sheet (initiates drag when pulled down at top)
 * @param {HTMLElement} [options.backdropEl=null] - Backdrop overlay whose opacity tracks drag progress
 * @param {() => boolean} options.getIsExpanded - Returns whether sheet is currently expanded
 * @param {(expanded: boolean) => void} options.onExpandChange - State callback when sheet expands or collapses
 * @param {number} [options.peekHeight=46] - Height visible when docked in px
 * @returns {{ destroy: () => void, isJustDragged: () => boolean }}
 */
export function bindBottomSheetDrag({
  sheetEl,
  dragHandles = [],
  scrollEl = null,
  backdropEl = null,
  getIsExpanded,
  onExpandChange,
  peekHeight = 46,
}) {
  if (!sheetEl) {
    return {
      destroy: () => {},
      isJustDragged: () => false,
    };
  }

  let isDragging = false;
  let hasMoved = false;
  let justDragged = false;
  let startY = 0;
  let startX = 0;
  let startTime = 0;
  let startTranslateY = 0;
  let currentTravel = 0;
  let snapTimeoutId = null;

  function calculateTravel() {
    const sheetHeight = sheetEl.offsetHeight || 0;
    if (sheetHeight > peekHeight) {
      return sheetHeight - peekHeight;
    }
    const winHeight = typeof window !== 'undefined' ? window.innerHeight : 667;
    return Math.max(100, winHeight - 58 - peekHeight);
  }

  function cleanupInlineStyles() {
    if (!sheetEl) return;
    sheetEl.style.transform = '';
    sheetEl.style.transition = '';
    sheetEl.style.willChange = '';
    if (backdropEl) {
      backdropEl.style.opacity = '';
      backdropEl.style.transition = '';
      backdropEl.style.visibility = '';
      backdropEl.style.pointerEvents = '';
    }
  }

  function startDrag(y, x) {
    if (snapTimeoutId) {
      clearTimeout(snapTimeoutId);
      snapTimeoutId = null;
    }

    startY = y;
    startX = x;
    startTime = Date.now();
    currentTravel = calculateTravel();
    const isExpanded = typeof getIsExpanded === 'function' ? getIsExpanded() : false;
    startTranslateY = isExpanded ? 0 : currentTravel;

    isDragging = true;
    hasMoved = false;
    justDragged = false;

    sheetEl.style.transition = 'none';
    sheetEl.style.willChange = 'transform';
    if (backdropEl) {
      backdropEl.style.transition = 'none';
      backdropEl.style.visibility = 'visible';
    }
  }

  function moveDrag(y, x, event) {
    if (!isDragging) return;

    const deltaY = y - startY;
    const deltaX = Math.abs(x - startX);

    if (!hasMoved) {
      // Si el movimiento es horizontal preponderante, ignorar para no bloquear gestos horizontales
      if (deltaX > Math.abs(deltaY) + 5) {
        cancelDrag();
        return;
      }
      if (Math.abs(deltaY) > 4) {
        hasMoved = true;
        justDragged = true;
      }
    }

    if (!hasMoved) return;

    let targetY = startTranslateY + deltaY;

    // Resistencia elástica en los extremos
    if (targetY < 0) {
      // Arrastre por encima del tope superior
      targetY = targetY * 0.22;
    } else if (targetY > currentTravel) {
      // Arrastre por debajo del tope inferior
      targetY = currentTravel + (targetY - currentTravel) * 0.22;
    }

    // Seguir el dedo en tiempo real (1:1) acelerado por GPU
    sheetEl.style.transform = `translate3d(0, ${targetY.toFixed(1)}px, 0)`;

    // Opacidad proporcional del backdrop siguiendo el dedo
    if (backdropEl && currentTravel > 0) {
      const progress = Math.max(0, Math.min(1, 1 - (targetY / currentTravel)));
      backdropEl.style.opacity = progress.toFixed(3);
      backdropEl.style.pointerEvents = progress > 0.35 ? 'auto' : 'none';
    }

    if (event && event.cancelable) {
      event.preventDefault();
    }
  }

  function endDrag(y) {
    if (!isDragging) return;
    isDragging = false;

    if (!hasMoved) {
      cleanupInlineStyles();
      return;
    }

    const deltaY = y - startY;
    const deltaTime = Math.max(1, Date.now() - startTime);
    const velocity = deltaY / deltaTime; // px/ms

    let shouldExpand = false;
    if (velocity < -0.32) {
      // Fling rápido hacia arriba -> expandir
      shouldExpand = true;
    } else if (velocity > 0.32) {
      // Fling rápido hacia abajo -> colapsar
      shouldExpand = false;
    } else {
      // Movimiento pausado: evaluar si superó el umbral del 35% hacia la meta
      const currentPos = startTranslateY + deltaY;
      shouldExpand = currentPos < currentTravel * 0.65;
    }

    snapTo(shouldExpand);
  }

  function cancelDrag() {
    if (!isDragging) return;
    isDragging = false;
    if (!hasMoved) {
      cleanupInlineStyles();
      return;
    }
    const isExpanded = typeof getIsExpanded === 'function' ? getIsExpanded() : false;
    snapTo(isExpanded);
  }

  function snapTo(shouldExpand) {
    const finalY = shouldExpand ? 0 : currentTravel;

    sheetEl.style.transition = 'transform 320ms cubic-bezier(0.16, 1, 0.3, 1), box-shadow 320ms ease, background 320ms ease, border-color 320ms ease';
    sheetEl.style.transform = `translate3d(0, ${finalY}px, 0)`;

    if (backdropEl) {
      backdropEl.style.transition = 'opacity 320ms cubic-bezier(0.16, 1, 0.3, 1)';
      backdropEl.style.opacity = shouldExpand ? '1' : '0';
      backdropEl.style.pointerEvents = shouldExpand ? 'auto' : 'none';
    }

    if (typeof onExpandChange === 'function') {
      onExpandChange(shouldExpand);
    }

    if (snapTimeoutId) {
      clearTimeout(snapTimeoutId);
    }

    snapTimeoutId = setTimeout(() => {
      cleanupInlineStyles();
      // Retener justDragged brevemente para evitar que el 'click' sintético del touchend re-dispare el toggle
      setTimeout(() => {
        justDragged = false;
      }, 60);
      snapTimeoutId = null;
    }, 330);
  }

  // --- Suscripción a eventos táctiles en dragHandles ---
  const listenersToClean = [];

  function registerHandle(handle) {
    if (!handle || typeof handle.addEventListener !== 'function') return;

    const onTouchStart = (e) => {
      if (e.touches && e.touches.length === 1) {
        startDrag(e.touches[0].clientY, e.touches[0].clientX);
      }
    };

    const onTouchMove = (e) => {
      if (!isDragging || !e.touches || e.touches.length !== 1) return;
      moveDrag(e.touches[0].clientY, e.touches[0].clientX, e);
    };

    const onTouchEnd = (e) => {
      if (!isDragging) return;
      const touch = (e.changedTouches && e.changedTouches[0]) || (e.touches && e.touches[0]);
      const endY = touch ? touch.clientY : startY;
      endDrag(endY);
    };

    const onTouchCancel = () => {
      cancelDrag();
    };

    handle.addEventListener('touchstart', onTouchStart, { passive: true });
    handle.addEventListener('touchmove', onTouchMove, { passive: false });
    handle.addEventListener('touchend', onTouchEnd, { passive: true });
    handle.addEventListener('touchcancel', onTouchCancel, { passive: true });

    listenersToClean.push(() => {
      handle.removeEventListener('touchstart', onTouchStart);
      handle.removeEventListener('touchmove', onTouchMove);
      handle.removeEventListener('touchend', onTouchEnd);
      handle.removeEventListener('touchcancel', onTouchCancel);
    });
  }

  dragHandles.forEach(registerHandle);

  // --- Soporte en scrollEl: arrastrar hacia abajo desde la parte superior de la lista expandida ---
  if (scrollEl && typeof scrollEl.addEventListener === 'function') {
    let scrollTouchStartY = 0;
    let scrollTouchStartX = 0;
    let isScrollPullingDown = false;

    const onScrollTouchStart = (e) => {
      if (e.touches && e.touches.length === 1) {
        scrollTouchStartY = e.touches[0].clientY;
        scrollTouchStartX = e.touches[0].clientX;
        isScrollPullingDown = false;
      }
    };

    const onScrollTouchMove = (e) => {
      if (!e.touches || e.touches.length !== 1) return;
      const isExpanded = typeof getIsExpanded === 'function' ? getIsExpanded() : false;
      if (!isExpanded) return;

      const currentY = e.touches[0].clientY;
      const currentX = e.touches[0].clientX;
      const deltaY = currentY - scrollTouchStartY;

      // Si está en la parte superior del scroll y el usuario arrastra hacia abajo
      const isAtTop = scrollEl.scrollTop <= 0;
      if (isAtTop && deltaY > 6 && !isDragging) {
        isScrollPullingDown = true;
        startDrag(currentY, currentX);
      }

      if (isScrollPullingDown && isDragging) {
        moveDrag(currentY, currentX, e);
      }
    };

    const onScrollTouchEnd = (e) => {
      if (isScrollPullingDown && isDragging) {
        isScrollPullingDown = false;
        const touch = (e.changedTouches && e.changedTouches[0]) || (e.touches && e.touches[0]);
        const endY = touch ? touch.clientY : scrollTouchStartY;
        endDrag(endY);
      }
      isScrollPullingDown = false;
    };

    const onScrollTouchCancel = () => {
      if (isScrollPullingDown) {
        isScrollPullingDown = false;
        cancelDrag();
      }
    };

    scrollEl.addEventListener('touchstart', onScrollTouchStart, { passive: true });
    scrollEl.addEventListener('touchmove', onScrollTouchMove, { passive: false });
    scrollEl.addEventListener('touchend', onScrollTouchEnd, { passive: true });
    scrollEl.addEventListener('touchcancel', onScrollTouchCancel, { passive: true });

    listenersToClean.push(() => {
      scrollEl.removeEventListener('touchstart', onScrollTouchStart);
      scrollEl.removeEventListener('touchmove', onScrollTouchMove);
      scrollEl.removeEventListener('touchend', onScrollTouchEnd);
      scrollEl.removeEventListener('touchcancel', onScrollTouchCancel);
    });
  }

  function destroy() {
    if (snapTimeoutId) {
      clearTimeout(snapTimeoutId);
      snapTimeoutId = null;
    }
    listenersToClean.forEach((cleanup) => cleanup());
    listenersToClean.length = 0;
    cleanupInlineStyles();
  }

  return {
    destroy,
    isJustDragged: () => justDragged,
  };
}
