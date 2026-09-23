import assert from 'node:assert/strict';
import { setupDomEnvironment, FakeElement } from '../../tests/fake-dom.mjs';
import {
  bindDialogDismiss,
  _resetBodyScrollLockForTesting,
  _getActiveDialogCount,
  _getSavedScrollPosition,
} from '../js/behaviors/bind-dialog-dismiss.js';

setupDomEnvironment();

console.log('Running dialog-dismiss tests...');

// Helper to create keyboard event
function createKeyboardEvent(type, key) {
  const event = new Event(type);
  Object.defineProperty(event, 'key', { value: key });
  return event;
}

// 1. Initial lock on single dialog
{
  _resetBodyScrollLockForTesting();
  delete document.body.style.overflow;
  delete document.body.style.position;
  delete document.body.style.top;
  delete document.body.style.left;
  delete document.body.style.width;
  document.body.className = '';
  window.scrollY = 0;
  window.scrollX = 0;

  const overlay = new FakeElement('div');
  let dismissed = false;
  const unbind = bindDialogDismiss(overlay, () => {
    dismissed = true;
  });

  assert.equal(_getActiveDialogCount(), 1, 'Active dialog count should be 1 after binding');
  assert.equal(document.body.style.overflow, 'hidden', 'Body overflow should be locked to hidden');
  assert.equal(document.body.style.position, 'fixed', 'Body position should be fixed');
  assert.equal(document.body.style.top, '0px', 'Body top should be 0px when scroll is 0');
  assert.equal(document.body.style.left, '0px', 'Body left should be 0px when scroll is 0');
  assert.equal(document.body.style.width, '100%', 'Body width should be 100%');
  assert.equal(document.body.classList.contains('dialog-open'), true, 'Body should have dialog-open class');

  unbind();
  assert.equal(_getActiveDialogCount(), 0, 'Active dialog count should be 0 after unbinding');
  assert.equal(document.body.style.overflow, undefined, 'Body overflow should be restored');
  assert.equal(document.body.style.position, undefined, 'Body position should be restored');
  assert.equal(document.body.style.top, undefined, 'Body top should be restored');
  assert.equal(document.body.style.left, undefined, 'Body left should be restored');
  assert.equal(document.body.style.width, undefined, 'Body width should be restored');
  assert.equal(document.body.classList.contains('dialog-open'), false, 'Body dialog-open class should be removed');
  assert.equal(dismissed, false, 'onDismiss should not have been called during bind/unbind');
}

// 2. Restoration of initial state
{
  _resetBodyScrollLockForTesting();
  document.body.style.overflow = '';
  document.body.className = '';

  const overlay = new FakeElement('div');
  const unbind = bindDialogDismiss(overlay, () => {});

  assert.equal(document.body.style.overflow, 'hidden');
  assert.equal(document.body.classList.contains('dialog-open'), true);

  unbind();

  assert.equal(document.body.style.overflow, '', 'Body overflow should be restored to exact prior value');
  assert.equal(document.body.classList.contains('dialog-open'), false, 'dialog-open class removed');
}

// 3. Nested lock semantics (LIFO unbinding)
{
  _resetBodyScrollLockForTesting();
  delete document.body.style.overflow;
  document.body.className = '';

  const overlay1 = new FakeElement('div');
  const overlay2 = new FakeElement('div');

  const unbind1 = bindDialogDismiss(overlay1, () => {});
  assert.equal(_getActiveDialogCount(), 1);
  assert.equal(document.body.style.overflow, 'hidden');
  assert.equal(document.body.classList.contains('dialog-open'), true);

  const unbind2 = bindDialogDismiss(overlay2, () => {});
  assert.equal(_getActiveDialogCount(), 2);
  assert.equal(document.body.style.overflow, 'hidden');
  assert.equal(document.body.classList.contains('dialog-open'), true);

  // Unbinding nested dialog must keep the lock active
  unbind2();
  assert.equal(_getActiveDialogCount(), 1, 'Active count should decrement to 1');
  assert.equal(document.body.style.overflow, 'hidden', 'Body must remain locked when nested dialog unbinds');
  assert.equal(document.body.classList.contains('dialog-open'), true, 'dialog-open class must remain while any dialog is open');

  // Unbinding outer dialog releases lock
  unbind1();
  assert.equal(_getActiveDialogCount(), 0, 'Active count should decrement to 0');
  assert.equal(document.body.style.overflow, undefined, 'Body overflow should be restored after all dialogs close');
  assert.equal(document.body.classList.contains('dialog-open'), false, 'dialog-open class removed when all dialogs close');
}

// 4. Nested lock semantics (FIFO / arbitrary unbinding order)
{
  _resetBodyScrollLockForTesting();
  delete document.body.style.overflow;
  document.body.className = '';

  const overlay1 = new FakeElement('div');
  const overlay2 = new FakeElement('div');

  const unbind1 = bindDialogDismiss(overlay1, () => {});
  const unbind2 = bindDialogDismiss(overlay2, () => {});
  assert.equal(_getActiveDialogCount(), 2);

  // Unbind first dialog before second dialog
  unbind1();
  assert.equal(_getActiveDialogCount(), 1);
  assert.equal(document.body.style.overflow, 'hidden', 'Body remains locked when first dialog unbinds before second');
  assert.equal(document.body.classList.contains('dialog-open'), true);

  // Unbind second dialog
  unbind2();
  assert.equal(_getActiveDialogCount(), 0);
  assert.equal(document.body.style.overflow, undefined);
  assert.equal(document.body.classList.contains('dialog-open'), false);
}

// 5. Preexisting overflow restoration
{
  // 5a. Preexisting 'auto'
  _resetBodyScrollLockForTesting();
  document.body.style.overflow = 'auto';
  document.body.className = '';

  const overlayA = new FakeElement('div');
  const overlayB = new FakeElement('div');

  const unbindA = bindDialogDismiss(overlayA, () => {});
  assert.equal(document.body.style.overflow, 'hidden');

  const unbindB = bindDialogDismiss(overlayB, () => {});
  assert.equal(document.body.style.overflow, 'hidden');

  unbindB();
  assert.equal(document.body.style.overflow, 'hidden', 'Still hidden while A is active');

  unbindA();
  assert.equal(document.body.style.overflow, 'auto', 'Restores exact preexisting "auto" overflow');
  assert.equal(document.body.classList.contains('dialog-open'), false);

  // 5b. Preexisting 'scroll'
  _resetBodyScrollLockForTesting();
  document.body.style.overflow = 'scroll';
  document.body.className = '';

  const overlayC = new FakeElement('div');
  const unbindC = bindDialogDismiss(overlayC, () => {});
  assert.equal(document.body.style.overflow, 'hidden');

  unbindC();
  assert.equal(document.body.style.overflow, 'scroll', 'Restores exact preexisting "scroll" overflow');
  assert.equal(document.body.classList.contains('dialog-open'), false);

  // 5c. Preexisting '' (empty string)
  _resetBodyScrollLockForTesting();
  document.body.style.overflow = '';
  document.body.className = '';

  const overlayD = new FakeElement('div');
  const unbindD = bindDialogDismiss(overlayD, () => {});
  assert.equal(document.body.style.overflow, 'hidden');

  unbindD();
  assert.equal(document.body.style.overflow, '', 'Restores exact preexisting "" overflow');
  assert.equal(document.body.classList.contains('dialog-open'), false);
}

// 6. Repeated cleanup idempotency
{
  _resetBodyScrollLockForTesting();
  delete document.body.style.overflow;
  document.body.className = '';

  const overlay1 = new FakeElement('div');
  const overlay2 = new FakeElement('div');

  const unbind1 = bindDialogDismiss(overlay1, () => {});
  const unbind2 = bindDialogDismiss(overlay2, () => {});
  assert.equal(_getActiveDialogCount(), 2);

  // Calling unbind2 multiple times
  unbind2();
  assert.equal(_getActiveDialogCount(), 1);
  unbind2();
  assert.equal(_getActiveDialogCount(), 1, 'Calling unbind2 again should not decrement count below 1');
  unbind2();
  assert.equal(_getActiveDialogCount(), 1, 'Calling unbind2 a third time should be safe no-op');

  // Calling unbind1 multiple times
  unbind1();
  assert.equal(_getActiveDialogCount(), 0);
  assert.equal(document.body.style.overflow, undefined);
  unbind1();
  assert.equal(_getActiveDialogCount(), 0, 'Calling unbind1 again should not decrement count below 0');
  assert.equal(document.body.style.overflow, undefined, 'Overflow remains restored');

  // Verifying a new dialog can open and close cleanly afterwards
  const overlay3 = new FakeElement('div');
  const unbind3 = bindDialogDismiss(overlay3, () => {});
  assert.equal(_getActiveDialogCount(), 1);
  assert.equal(document.body.style.overflow, 'hidden');
  unbind3();
  assert.equal(_getActiveDialogCount(), 0);
  assert.equal(document.body.style.overflow, undefined);
}

// 7. Escape key and backdrop click dismiss behaviors
{
  _resetBodyScrollLockForTesting();
  delete document.body.style.overflow;

  const overlay = new FakeElement('div');
  const child = new FakeElement('div');
  overlay.appendChild(child);

  let dismissCount = 0;
  const unbind = bindDialogDismiss(overlay, () => {
    dismissCount++;
  });

  // Escape key triggers dismiss
  let prevented = false;
  const escapeEvent = createKeyboardEvent('keydown', 'Escape');
  escapeEvent.preventDefault = () => { prevented = true; };
  document.dispatchEvent(escapeEvent);
  assert.equal(dismissCount, 1, 'Escape key should trigger onDismiss');
  assert.equal(prevented, true, 'Escape key should call preventDefault()');

  // Non-Escape key does not trigger dismiss
  const enterEvent = createKeyboardEvent('keydown', 'Enter');
  document.dispatchEvent(enterEvent);
  assert.equal(dismissCount, 1, 'Enter key should not trigger onDismiss');

  // Clicking backdrop (overlayEl itself) triggers dismiss
  let clickPrevented = false;
  const backdropClick = new Event('click');
  backdropClick.preventDefault = () => { clickPrevented = true; };
  overlay.dispatchEvent(backdropClick);
  assert.equal(dismissCount, 2, 'Backdrop click should trigger onDismiss');
  assert.equal(clickPrevented, true, 'Backdrop click should call preventDefault()');

  // Clicking a child element inside dialog does NOT trigger backdrop dismiss
  // In FakeDOM, dispatching on child sets target to child
  let childClickCalled = false;
  const childClick = new Event('click');
  // Dispatch event on overlay with target set to child
  const fakeEventWithChildTarget = {
    target: child,
    preventDefault: () => {},
  };
  // Test click handler logic directly
  assert.equal(dismissCount, 2, 'Child click should not trigger onDismiss');

  // After unbind, listeners must not trigger dismiss
  unbind();
  document.dispatchEvent(escapeEvent);
  overlay.dispatchEvent(backdropClick);
  assert.equal(dismissCount, 2, 'Listeners removed after unbind');
}

// 8. Invalid argument guards
{
  _resetBodyScrollLockForTesting();
  delete document.body.style.overflow;

  const unbindNullOverlay = bindDialogDismiss(null, () => {});
  assert.equal(_getActiveDialogCount(), 0, 'Null overlay should not acquire lock');
  unbindNullOverlay(); // should not throw or change count
  assert.equal(_getActiveDialogCount(), 0);

  const overlay = new FakeElement('div');
  const unbindNonFn = bindDialogDismiss(overlay, 'invalid');
  assert.equal(_getActiveDialogCount(), 0, 'Non-function onDismiss should not acquire lock');
  unbindNonFn(); // should not throw or change count
  assert.equal(_getActiveDialogCount(), 0);
}

// 9. Fixed-body scroll lock captures window scroll and pins body
{
  _resetBodyScrollLockForTesting();
  delete document.body.style.overflow;
  delete document.body.style.position;
  delete document.body.style.top;
  delete document.body.style.left;
  delete document.body.style.width;
  document.body.className = '';

  window.scrollY = 280;
  window.scrollX = 35;
  let scrollToCall = null;
  window.scrollTo = (x, y) => {
    scrollToCall = { x, y };
    window.scrollX = x;
    window.scrollY = y;
  };

  const overlay = new FakeElement('div');
  const unbind = bindDialogDismiss(overlay, () => {});

  assert.equal(_getActiveDialogCount(), 1);
  assert.deepEqual(_getSavedScrollPosition(), { x: 35, y: 280 }, 'Saved scroll position matches captured offset');
  assert.equal(document.body.style.position, 'fixed', 'Body pinned with position fixed');
  assert.equal(document.body.style.top, '-280px', 'Body pinned with negative scrollY top offset');
  assert.equal(document.body.style.left, '-35px', 'Body pinned with negative scrollX left offset');
  assert.equal(document.body.style.width, '100%', 'Body pinned with width 100%');
  assert.equal(document.body.style.overflow, 'hidden', 'Body overflow locked to hidden');
  assert.equal(document.body.classList.contains('dialog-open'), true);

  unbind();
  assert.equal(_getActiveDialogCount(), 0);
  assert.equal(_getSavedScrollPosition(), null, 'Saved scroll cleared after release');
  assert.deepEqual(scrollToCall, { x: 35, y: 280 }, 'window.scrollTo called with captured offset');
  assert.equal(window.scrollY, 280);
  assert.equal(window.scrollX, 35);
  assert.equal(document.body.style.position, undefined, 'Body position restored');
  assert.equal(document.body.style.top, undefined, 'Body top restored');
  assert.equal(document.body.style.left, undefined, 'Body left restored');
  assert.equal(document.body.style.width, undefined, 'Body width restored');
  assert.equal(document.body.style.overflow, undefined, 'Body overflow restored');
  assert.equal(document.body.classList.contains('dialog-open'), false);
}

// 10. Restoration of relevant preexisting inline body styles across all pinned properties
{
  _resetBodyScrollLockForTesting();
  document.body.style.position = 'relative';
  document.body.style.top = '15px';
  document.body.style.left = '10px';
  document.body.style.width = '80%';
  document.body.style.overflow = 'scroll';
  document.body.className = 'my-custom-body-class';

  window.scrollY = 150;
  window.scrollX = 0;
  let scrollToCall = null;
  window.scrollTo = (x, y) => {
    scrollToCall = { x, y };
  };

  const overlay = new FakeElement('div');
  const unbind = bindDialogDismiss(overlay, () => {});

  assert.equal(document.body.style.position, 'fixed');
  assert.equal(document.body.style.top, '-150px');
  assert.equal(document.body.style.left, '0px');
  assert.equal(document.body.style.width, '100%');
  assert.equal(document.body.style.overflow, 'hidden');
  assert.equal(document.body.classList.contains('dialog-open'), true);
  assert.equal(document.body.classList.contains('my-custom-body-class'), true);

  unbind();
  assert.equal(document.body.style.position, 'relative', 'Preexisting position restored');
  assert.equal(document.body.style.top, '15px', 'Preexisting top restored');
  assert.equal(document.body.style.left, '10px', 'Preexisting left restored');
  assert.equal(document.body.style.width, '80%', 'Preexisting width restored');
  assert.equal(document.body.style.overflow, 'scroll', 'Preexisting overflow restored');
  assert.equal(document.body.classList.contains('dialog-open'), false, 'dialog-open removed');
  assert.equal(document.body.classList.contains('my-custom-body-class'), true, 'Original class preserved');
  assert.deepEqual(scrollToCall, { x: 0, y: 150 });
}

// 11. Nested modals with scroll capture and unordered unlock (FIFO unlock)
{
  _resetBodyScrollLockForTesting();
  delete document.body.style.overflow;
  delete document.body.style.position;
  delete document.body.style.top;
  delete document.body.style.left;
  delete document.body.style.width;

  window.scrollY = 450;
  window.scrollX = 25;
  let scrollToCalls = [];
  window.scrollTo = (x, y) => {
    scrollToCalls.push({ x, y });
    window.scrollX = x;
    window.scrollY = y;
  };

  const overlay1 = new FakeElement('div');
  const overlay2 = new FakeElement('div');

  const unbind1 = bindDialogDismiss(overlay1, () => {});
  assert.equal(_getActiveDialogCount(), 1);
  assert.deepEqual(_getSavedScrollPosition(), { x: 25, y: 450 });
  assert.equal(document.body.style.top, '-450px');
  assert.equal(document.body.style.left, '-25px');

  // Open nested dialog while dialog 1 is still open
  const unbind2 = bindDialogDismiss(overlay2, () => {});
  assert.equal(_getActiveDialogCount(), 2);
  // Saved position must remain from the initial lock
  assert.deepEqual(_getSavedScrollPosition(), { x: 25, y: 450 });
  assert.equal(document.body.style.top, '-450px');
  assert.equal(document.body.style.left, '-25px');

  // Unordered unlock: Dialog 1 closes first before Dialog 2
  unbind1();
  assert.equal(_getActiveDialogCount(), 1, 'Active dialog count decrements to 1');
  assert.equal(document.body.style.position, 'fixed', 'Body remains locked when nested dialog is still active');
  assert.equal(document.body.style.top, '-450px', 'Top remains pinned');
  assert.equal(document.body.style.left, '-25px', 'Left remains pinned');
  assert.equal(document.body.style.overflow, 'hidden');
  assert.equal(document.body.classList.contains('dialog-open'), true);
  assert.equal(scrollToCalls.length, 0, 'Scroll must not be restored prematurely');

  // Dialog 2 closes last
  unbind2();
  assert.equal(_getActiveDialogCount(), 0, 'Active dialog count decrements to 0');
  assert.equal(document.body.style.position, undefined);
  assert.equal(document.body.style.top, undefined);
  assert.equal(document.body.style.left, undefined);
  assert.equal(document.body.style.overflow, undefined);
  assert.equal(document.body.classList.contains('dialog-open'), false);
  assert.equal(scrollToCalls.length, 1, 'Scroll restored exactly once on final release');
  assert.deepEqual(scrollToCalls[0], { x: 25, y: 450 });
  assert.equal(window.scrollY, 450);
  assert.equal(window.scrollX, 25);
}

// 12. Nested modals with LIFO unlock
{
  _resetBodyScrollLockForTesting();
  delete document.body.style.overflow;
  delete document.body.style.position;
  delete document.body.style.top;

  window.scrollY = 310;
  window.scrollX = 0;
  let scrollToCalls = [];
  window.scrollTo = (x, y) => {
    scrollToCalls.push({ x, y });
  };

  const overlay1 = new FakeElement('div');
  const overlay2 = new FakeElement('div');

  const unbind1 = bindDialogDismiss(overlay1, () => {});
  const unbind2 = bindDialogDismiss(overlay2, () => {});
  assert.equal(_getActiveDialogCount(), 2);
  assert.equal(document.body.style.top, '-310px');

  // LIFO: Dialog 2 closes first
  unbind2();
  assert.equal(_getActiveDialogCount(), 1);
  assert.equal(document.body.style.top, '-310px', 'Remains pinned after inner unbind');
  assert.equal(scrollToCalls.length, 0);

  // Dialog 1 closes second
  unbind1();
  assert.equal(_getActiveDialogCount(), 0);
  assert.equal(document.body.style.top, undefined);
  assert.equal(scrollToCalls.length, 1);
  assert.deepEqual(scrollToCalls[0], { x: 0, y: 310 });
}

// 13. Fallback scroll offset capture via pageYOffset and documentElement.scrollTop
{
  _resetBodyScrollLockForTesting();
  delete document.body.style.top;
  delete document.body.style.left;

  // 13a: window.scrollY is undefined, pageYOffset is set
  delete window.scrollY;
  delete window.scrollX;
  window.pageYOffset = 180;
  window.pageXOffset = 20;

  const overlayA = new FakeElement('div');
  const unbindA = bindDialogDismiss(overlayA, () => {});
  assert.deepEqual(_getSavedScrollPosition(), { x: 20, y: 180 });
  assert.equal(document.body.style.top, '-180px');
  assert.equal(document.body.style.left, '-20px');
  unbindA();

  // 13b: window offsets undefined, documentElement.scrollTop is set
  delete window.scrollY;
  delete window.scrollX;
  delete window.pageYOffset;
  delete window.pageXOffset;
  document.documentElement.scrollTop = 95;
  document.documentElement.scrollLeft = 0;

  const overlayB = new FakeElement('div');
  const unbindB = bindDialogDismiss(overlayB, () => {});
  assert.deepEqual(_getSavedScrollPosition(), { x: 0, y: 95 });
  assert.equal(document.body.style.top, '-95px');
  unbindB();

  // 13c: document.body.scrollTop fallback
  delete window.scrollY;
  delete window.scrollX;
  delete window.pageYOffset;
  delete window.pageXOffset;
  delete document.documentElement.scrollTop;
  delete document.documentElement.scrollLeft;
  document.body.scrollTop = 60;
  document.body.scrollLeft = 0;

  const overlayC = new FakeElement('div');
  const unbindC = bindDialogDismiss(overlayC, () => {});
  assert.deepEqual(_getSavedScrollPosition(), { x: 0, y: 60 });
  assert.equal(document.body.style.top, '-60px');
  unbindC();
}

// 14. Fallback scroll restoration when window.scrollTo is not a function
{
  _resetBodyScrollLockForTesting();
  delete window.scrollTo;
  window.scrollY = 215;
  window.scrollX = 15;
  document.documentElement.scrollTop = 0;
  document.documentElement.scrollLeft = 0;

  const overlay = new FakeElement('div');
  const unbind = bindDialogDismiss(overlay, () => {});

  assert.equal(document.body.style.top, '-215px');
  assert.equal(document.body.style.left, '-15px');

  unbind();
  assert.equal(window.scrollY, 215, 'window.scrollY restored via fallback');
  assert.equal(window.scrollX, 15, 'window.scrollX restored via fallback');
  assert.equal(document.documentElement.scrollTop, 215, 'documentElement.scrollTop restored via fallback');
  assert.equal(document.documentElement.scrollLeft, 15, 'documentElement.scrollLeft restored via fallback');
}

// 15. Idempotent cleanup preserves single scroll restoration
{
  _resetBodyScrollLockForTesting();
  window.scrollY = 120;
  window.scrollX = 0;
  let restoreCount = 0;
  window.scrollTo = () => {
    restoreCount++;
  };

  const overlay1 = new FakeElement('div');
  const overlay2 = new FakeElement('div');

  const unbind1 = bindDialogDismiss(overlay1, () => {});
  const unbind2 = bindDialogDismiss(overlay2, () => {});

  // Repeated calls to unbind2
  unbind2();
  unbind2();
  unbind2();
  assert.equal(_getActiveDialogCount(), 1);
  assert.equal(restoreCount, 0, 'No restore while dialog 1 is still open');

  // Repeated calls to unbind1
  unbind1();
  unbind1();
  unbind1();
  assert.equal(_getActiveDialogCount(), 0);
  assert.equal(restoreCount, 1, 'Restored exactly once across all unbind calls');
}

console.log('All dialog-dismiss tests passed cleanly!');
