import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const parseRouteModulePath = resolve(process.cwd(), 'frontend/js/router/parse-route.js');
const { parseRoute } = await import(pathToFileURL(parseRouteModulePath).href);

const navigateModulePath = resolve(process.cwd(), 'frontend/js/router/navigate.js');
const {
  toPresentations,
  toPresentation,
  toEvents,
  toEvent,
  toPublicTicket,
  toDoor,
} = await import(pathToFileURL(navigateModulePath).href);

console.log('Running Router Unit Tests for Shirley Show Ticketing...');

// 1. Existing presentation catalog & detail routes
{
  assert.deepEqual(parseRoute(''), { view: 'presentations', params: {} });
  assert.deepEqual(parseRoute('#'), { view: 'presentations', params: {} });
  assert.deepEqual(parseRoute('#/'), { view: 'presentations', params: {} });
  assert.deepEqual(parseRoute('#/presentations'), { view: 'presentations', params: {} });
  assert.deepEqual(parseRoute('presentations'), { view: 'presentations', params: {} });

  assert.deepEqual(parseRoute('#/presentations/pres-1'), {
    view: 'presentation-detail',
    params: { id: 'pres-1' },
  });

  assert.deepEqual(parseRoute('#/presentations/pres%20con%20espacio'), {
    view: 'presentation-detail',
    params: { id: 'pres con espacio' },
  });
}

// 2. New ticketing routes: #/eventos, #/eventos/:id, #/ticket/:token, #/puerta/:id
{
  // Catalog of events
  assert.deepEqual(parseRoute('#/eventos'), {
    view: 'events',
    params: {},
  });
  assert.deepEqual(parseRoute('eventos'), {
    view: 'events',
    params: {},
  });
  assert.deepEqual(parseRoute('#/eventos/'), {
    view: 'events',
    params: {},
  });

  // Event detail with id
  assert.deepEqual(parseRoute('#/eventos/evt-100'), {
    view: 'event-detail',
    params: { id: 'evt-100' },
  });
  assert.deepEqual(parseRoute('#/eventos/cena%20show'), {
    view: 'event-detail',
    params: { id: 'cena show' },
  });

  // Public ticket view with scan_token
  assert.deepEqual(parseRoute('#/ticket/st_1234567890abcdef'), {
    view: 'public-ticket',
    params: { token: 'st_1234567890abcdef' },
  });

  // Door / Check-in view with event id
  assert.deepEqual(parseRoute('#/puerta/evt-100'), {
    view: 'door',
    params: { id: 'evt-100' },
  });
}

// 3. Fallbacks: unknown routes and malformed URI decoding
{
  assert.deepEqual(parseRoute('#/unknown-route'), {
    view: 'presentations',
    params: {},
    fallback: true,
  });

  assert.deepEqual(parseRoute('#/eventos/malformed%E0%A4A'), {
    view: 'presentations',
    params: {},
    fallback: true,
  });

  assert.deepEqual(parseRoute('#/ticket/%E0%A4A'), {
    view: 'presentations',
    params: {},
    fallback: true,
  });
}

// 4. Navigation helper methods
{
  // Mock window.location
  globalThis.window = {
    location: {
      hash: '',
    },
  };

  toEvents();
  assert.equal(globalThis.window.location.hash, '#/eventos');

  toEvent('evt-42');
  assert.equal(globalThis.window.location.hash, '#/eventos/evt-42');

  toPublicTicket('st_test_token');
  assert.equal(globalThis.window.location.hash, '#/ticket/st_test_token');

  toDoor('evt-42');
  assert.equal(globalThis.window.location.hash, '#/puerta/evt-42');

  toPresentations();
  assert.equal(globalThis.window.location.hash, '#/presentations');

  toPresentation('pres-99');
  assert.equal(globalThis.window.location.hash, '#/presentations/pres-99');
}

console.log('All Router Unit Tests passed successfully!');
