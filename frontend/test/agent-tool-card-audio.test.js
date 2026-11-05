import assert from 'node:assert/strict';
import { setupDomEnvironment } from '../../tests/fake-dom.mjs';
import { createAgentToolCard } from '../js/components/molecules/agent-tool-card.js';

setupDomEnvironment();

console.log('Running agent-tool-card audio actions tests...');

if (!globalThis.HTMLElement.prototype.click) {
  globalThis.HTMLElement.prototype.click = function () {
    this.dispatchEvent(new Event('click'));
  };
}

const TRACK_ACTION = {
  tool: 'add_track_from_staged',
  summary: 'Pista «Chiquitita (Coros)» añadida a la presentación',
  status: 'success',
  presentation_id: 'pres-1',
  details: { item_id: 'item-42', title: 'Chiquitita (Coros)' },
};

// 1. A successfully added track offers download and playback
{
  const card = createAgentToolCard({ action: TRACK_ACTION });

  const download = card.el.querySelector('.agent-tool-card-download');
  assert.ok(download, 'una pista añadida debe ofrecer descarga');

  const href = download.getAttribute('href') || '';
  assert.equal(
    href,
    '/api/tracks/item-42/audio?download=1',
    'la descarga apunta al audio de la pista añadida'
  );

  const audio = card.el.querySelector('.agent-tool-card-audio');
  assert.ok(audio, 'una pista añadida debe ofrecer reproducción');
  assert.equal(
    audio.getAttribute('src'),
    '/api/tracks/item-42/audio',
    'el reproductor apunta al audio de la pista añadida'
  );
  assert.ok(audio.hasAttribute('controls'), 'el reproductor expone sus controles');
}

// 2. Actions that did not add a track stay untouched
{
  const card = createAgentToolCard({
    action: { tool: 'renombrar_presentacion', summary: 'Renombrada', status: 'success' },
  });
  assert.equal(card.el.querySelector('.agent-tool-card-download'), null);
  assert.equal(card.el.querySelector('.agent-tool-card-audio'), null);
}

// 3. A failed action offers nothing to download
{
  const card = createAgentToolCard({
    action: { ...TRACK_ACTION, status: 'error', summary: 'Falló' },
  });
  assert.equal(card.el.querySelector('.agent-tool-card-download'), null);
}

// 4. A success without item_id must not build a broken link
{
  const card = createAgentToolCard({
    action: { tool: 'add_track_from_staged', summary: 'ok', status: 'success', details: {} },
  });
  assert.equal(card.el.querySelector('.agent-tool-card-download'), null);
}

console.log('agent-tool-card audio actions tests passed');
