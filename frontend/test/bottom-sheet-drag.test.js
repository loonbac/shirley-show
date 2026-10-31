import assert from 'node:assert/strict';
import { setupDomEnvironment, FakeElement } from '../../tests/fake-dom.mjs';
import { bindBottomSheetDrag } from '../js/behaviors/bind-bottom-sheet-drag.js';

setupDomEnvironment();

console.log('Running bind-bottom-sheet-drag tests...');

// Helper to create a touch event
function createTouchEvent(type, touches = []) {
  const event = new Event(type, { cancelable: true, bubbles: true });
  event.touches = touches;
  event.changedTouches = touches;
  return event;
}

// 1. Instantiation and cleanup with empty/null elements
{
  const result = bindBottomSheetDrag({});
  assert.equal(typeof result.destroy, 'function');
  assert.equal(typeof result.isJustDragged, 'function');
  assert.equal(result.isJustDragged(), false);
  result.destroy();
}

// 2. Dragging up from docked position tracks finger position in real time
{
  const sheet = new FakeElement('div');
  sheet.offsetHeight = 500;
  const handle = new FakeElement('div');
  const backdrop = new FakeElement('div');

  let expandedState = false;
  const drag = bindBottomSheetDrag({
    sheetEl: sheet,
    dragHandles: [handle],
    backdropEl: backdrop,
    getIsExpanded: () => expandedState,
    onExpandChange: (val) => {
      expandedState = val;
    },
    peekHeight: 50,
  });

  // Start touch at y = 600, x = 200
  handle.dispatchEvent(createTouchEvent('touchstart', [{ clientY: 600, clientX: 200 }]));
  assert.equal(sheet.style.transition, 'none', 'Transition should be disabled during active drag');
  assert.equal(sheet.style.willChange, 'transform');

  // Move finger up by 150px (to y = 450)
  handle.dispatchEvent(createTouchEvent('touchmove', [{ clientY: 450, clientX: 200 }]));
  assert.ok(sheet.style.transform.includes('translate3d'), 'Transform should follow finger movement');
  assert.ok(drag.isJustDragged(), 'Should be flagged as just dragged during move');

  // Release finger (slow drag past threshold)
  handle.dispatchEvent(createTouchEvent('touchend', [{ clientY: 450, clientX: 200 }]));
  assert.equal(expandedState, true, 'Sheet should snap to expanded when dragged past threshold');
  assert.ok(sheet.style.transition.includes('cubic-bezier'), 'Physics spring transition should be restored on snap');

  drag.destroy();
}

// 3. Fast flick down collapses sheet immediately
{
  const sheet = new FakeElement('div');
  sheet.offsetHeight = 500;
  const handle = new FakeElement('div');

  let expandedState = true;
  const drag = bindBottomSheetDrag({
    sheetEl: sheet,
    dragHandles: [handle],
    getIsExpanded: () => expandedState,
    onExpandChange: (val) => {
      expandedState = val;
    },
    peekHeight: 50,
  });

  // Start touch at y = 100
  handle.dispatchEvent(createTouchEvent('touchstart', [{ clientY: 100, clientX: 200 }]));

  // Quick move down by 50px
  handle.dispatchEvent(createTouchEvent('touchmove', [{ clientY: 150, clientX: 200 }]));

  // Release with downward motion
  handle.dispatchEvent(createTouchEvent('touchend', [{ clientY: 200, clientX: 200 }]));
  assert.equal(expandedState, false, 'Sheet should collapse on downward drag');

  drag.destroy();
}

// 4. Click without drag does not trigger snap or block click
{
  const sheet = new FakeElement('div');
  sheet.offsetHeight = 500;
  const handle = new FakeElement('div');

  let expandedState = false;
  const drag = bindBottomSheetDrag({
    sheetEl: sheet,
    dragHandles: [handle],
    getIsExpanded: () => expandedState,
    onExpandChange: (val) => {
      expandedState = val;
    },
    peekHeight: 50,
  });

  // Touch and immediate release without move
  handle.dispatchEvent(createTouchEvent('touchstart', [{ clientY: 300, clientX: 200 }]));
  handle.dispatchEvent(createTouchEvent('touchend', [{ clientY: 300, clientX: 200 }]));

  assert.equal(drag.isJustDragged(), false, 'Tap without movement should not be considered a drag');
  assert.equal(expandedState, false, 'Tap should not trigger snap');

  drag.destroy();
}

console.log('All bind-bottom-sheet-drag tests passed successfully.');
