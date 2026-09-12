import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { setupDomEnvironment } from '../../tests/fake-dom.mjs';

setupDomEnvironment();

const scannerModulePath = resolve(process.cwd(), 'frontend/js/modules/events/camera-qr-scanner.js');
const { isBarcodeDetectorSupported, createCameraQrScanner } = await import(pathToFileURL(scannerModulePath).href);

const doorModulePath = resolve(process.cwd(), 'frontend/js/modules/events/door-controller.js');
const { createDoorView, createDoorController } = await import(pathToFileURL(doorModulePath).href);

console.log('Running Door Mode and Camera QR Scanner tests...');

// 1. Camera QR Scanner Lifecycle & Fallback
{
  assert.equal(typeof isBarcodeDetectorSupported(), 'boolean');

  let tracksStopped = false;
  const mockTrack = {
    stop: () => {
      tracksStopped = true;
    },
  };
  const mockStream = {
    getTracks: () => [mockTrack],
  };

  globalThis.BarcodeDetector = class MockBarcodeDetector {
    constructor() {}
    async detect() {
      return [{ rawValue: 'a1b2c3d4e5f60718293a4b5c6d7e8f90' }];
    }
  };
  globalThis.window.BarcodeDetector = globalThis.BarcodeDetector;

  Object.defineProperty(globalThis.navigator, 'mediaDevices', {
    value: { getUserMedia: async () => mockStream },
    configurable: true,
    writable: true,
  });
  globalThis.window.navigator = globalThis.navigator;

  assert.equal(isBarcodeDetectorSupported(), true);

  let scannedCode = null;
  const fakeVideo = document.createElement('video');
  fakeVideo.readyState = 4; // HAVE_ENOUGH_DATA
  fakeVideo.play = async () => {};
  fakeVideo.pause = () => {};

  const scanner = createCameraQrScanner({
    videoEl: fakeVideo,
    onScan: (code) => {
      scannedCode = code;
    },
    scanIntervalMs: 20,
  });

  await scanner.start();
  assert.equal(scanner.isScanning(), true);

  // Wait for scan loop to fire
  await new Promise((r) => setTimeout(r, 60));
  assert.equal(scannedCode, 'a1b2c3d4e5f60718293a4b5c6d7e8f90');

  scanner.stop();
  assert.equal(scanner.isScanning(), false);
  assert.equal(tracksStopped, true, 'Scanner stop() must stop all media tracks');

  scanner.destroy();
}

// 2. Door View - 3 Distinct Feedback Banners (Valida, Ya Usada, Invalida)
{
  let submittedCode = null;
  const view = createDoorView({
    eventName: 'Noche Criolla',
    onSubmitCode: async (code) => {
      submittedCode = code;
    },
  });

  // Initial Idle State
  const idleBanner = view.el.querySelector('.door-banner-idle');
  assert.ok(idleBanner, 'Initial state must render idle banner');
  assert.ok(view.el.querySelector('.door-manual-input'), 'Manual input MUST always be present');

  // Outcome 1: Valida (Green banner)
  view.update({
    result: {
      resultado: 'Valida',
      nombre_comprador: 'Elena Torres',
      tipo_entrada: 'General',
      hora_ingreso: '2026-10-15T21:30:00Z',
    },
  });

  const validBanner = view.el.querySelector('.door-banner-valid');
  assert.ok(validBanner, 'Valida outcome MUST render .door-banner-valid');
  assert.ok(validBanner.textContent.includes('ENTRADA VÁLIDA'));
  assert.ok(validBanner.textContent.includes('Elena Torres'));
  assert.ok(validBanner.textContent.includes('General'));

  // Outcome 2: Ya Usada (Amber banner)
  view.update({
    result: {
      resultado: 'Ya Usada',
      hora_ingreso: '2026-10-15T21:15:00Z',
    },
  });

  const usedBanner = view.el.querySelector('.door-banner-used');
  assert.ok(usedBanner, 'Ya Usada outcome MUST render .door-banner-used');
  assert.ok(usedBanner.textContent.includes('YA FUE USADA'));

  // Outcome 3: Invalida (Red banner)
  view.update({
    result: {
      resultado: 'Invalida',
      motivo: 'Codigo pertenece a otro evento',
    },
  });

  const invalidBanner = view.el.querySelector('.door-banner-invalid');
  assert.ok(invalidBanner, 'Invalida outcome MUST render .door-banner-invalid');
  assert.ok(invalidBanner.textContent.includes('ENTRADA INVÁLIDA'));
  assert.ok(invalidBanner.textContent.includes('Codigo pertenece a otro evento'));

  view.destroy();
}

// 3. Door Controller Integration
{
  let checkedEventId = null;
  let checkedCode = null;

  const mockApi = {
    getEvent: async (id) => ({
      id,
      nombre: 'Gran Festival 2026',
    }),
    checkInTicket: async (eventId, codigo) => {
      checkedEventId = eventId;
      checkedCode = codigo;
      return {
        resultado: 'Valida',
        nombre_comprador: 'Mateo Quispe',
        tipo_entrada: 'VIP',
        hora_ingreso: '2026-10-15T22:00:00Z',
      };
    },
  };

  const controller = createDoorController({
    eventId: 'evt-door-123',
    api: mockApi,
  });

  assert.ok(controller.viewEl, 'Controller provides viewEl');

  // Submit code through form
  const input = controller.viewEl.querySelector('.door-manual-input');
  input.value = 'a1b2c3d4e5f60718293a4b5c6d7e8f90';

  const form = controller.viewEl.querySelector('form');
  form.dispatchEvent(new Event('submit'));

  await new Promise((r) => setTimeout(r, 20));

  assert.equal(checkedEventId, 'evt-door-123');
  assert.equal(checkedCode, 'a1b2c3d4e5f60718293a4b5c6d7e8f90');
  assert.ok(controller.viewEl.textContent.includes('Mateo Quispe'));

  controller.destroy();
}

console.log('Door Mode tests passed successfully.');
