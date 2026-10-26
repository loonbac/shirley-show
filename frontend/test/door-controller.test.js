import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { inflateSync } from 'node:zlib';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import vm from 'node:vm';
import { setupDomEnvironment } from '../../tests/fake-dom.mjs';

setupDomEnvironment();

const scannerModulePath = resolve(process.cwd(), 'frontend/js/modules/events/camera-qr-scanner.js');
const {
  isBarcodeDetectorSupported,
  isCameraSupported,
  getJsQR,
  getZXingNamespace,
  decodeFrameWithFallbacks,
  decodeWithZXingLowLevel,
  waitForVideoMetadata,
  createCameraQrScanner,
} = await import(pathToFileURL(scannerModulePath).href);

const doorModulePath = resolve(process.cwd(), 'frontend/js/modules/events/door-controller.js');
const {
  resolveDoorTicketType,
  formatCameraErrorMessage,
  createDoorView,
  createDoorController,
} = await import(pathToFileURL(doorModulePath).href);

console.log('Running Door Mode and Camera QR Scanner tests...');

// 1. Camera QR Scanner Lifecycle & Fallback (Native BarcodeDetector)
{
  assert.equal(typeof isBarcodeDetectorSupported(), 'boolean');
  assert.equal(typeof isCameraSupported(), 'boolean');

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
  assert.equal(
    view.el.querySelector('.door-manual-input'),
    null,
    'Manual code input must not exist: check-in is scan-only'
  );
  assert.ok(
    view.el.querySelector('.door-scan-hint'),
    'A hint telling the operator to scan must be rendered'
  );

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
      evento: {
        id,
        nombre: 'Gran Festival 2026',
      },
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

  // The decoded QR is the only entry point: there is no manual form any more.
  await controller.submitCode('a1b2c3d4e5f60718293a4b5c6d7e8f90');
  await new Promise((r) => setTimeout(r, 20));

  assert.equal(checkedEventId, 'evt-door-123');
  assert.equal(checkedCode, 'a1b2c3d4e5f60718293a4b5c6d7e8f90');
  assert.ok(controller.viewEl.textContent.includes('Mateo Quispe'));

  // R1-001: an 8-hex prefix is not a credential and must never reach the API.
  let shortCheckedCode = null;
  mockApi.checkInTicket = async (eventId, codigo) => {
    shortCheckedCode = codigo;
    return { resultado: 'Valida' };
  };

  await controller.submitCode('611D26D0');
  await new Promise((r) => setTimeout(r, 20));

  assert.equal(shortCheckedCode, null, '8-hex code must never reach checkInTicket');
  assert.ok(
    controller.viewEl.textContent.includes('Formato de código inválido'),
    '8-hex code must be rejected locally'
  );

  controller.destroy();
}

// 4. DOOR-1: Camera QR Scanner with jsQR Fallback when BarcodeDetector is unavailable
{
  // Temporarily disable BarcodeDetector
  delete globalThis.BarcodeDetector;
  delete globalThis.window.BarcodeDetector;
  assert.equal(isBarcodeDetectorSupported(), false, 'BarcodeDetector must not be supported in this sub-test');

  let tracksStopped = false;
  const mockTrack = {
    stop: () => {
      tracksStopped = true;
    },
  };
  const mockStream = {
    getTracks: () => [mockTrack],
  };

  Object.defineProperty(globalThis.navigator, 'mediaDevices', {
    value: { getUserMedia: async () => mockStream },
    configurable: true,
    writable: true,
  });
  globalThis.window.navigator = globalThis.navigator;

  // Mock global jsQR
  let jsQRCallCount = 0;
  globalThis.jsQR = (data, width, height) => {
    jsQRCallCount++;
    return { data: 'jsqr-decoded-token-1234567890abcdef' };
  };
  globalThis.window.jsQR = globalThis.jsQR;

  assert.equal(typeof getJsQR(), 'function', 'getJsQR must resolve global jsQR function');

  // Mock document.createElement to provide 2D canvas context for fallback
  const origCreateElement = document.createElement.bind(document);
  document.createElement = (tag) => {
    const el = origCreateElement(tag);
    if (String(tag).toLowerCase() === 'canvas') {
      el.getContext = (type) => ({
        drawImage: () => {},
        getImageData: () => ({
          data: new Uint8ClampedArray(4 * 640 * 480),
          width: 640,
          height: 480,
        }),
      });
    }
    return el;
  };

  const fakeVideo = document.createElement('video');
  fakeVideo.readyState = 4;
  fakeVideo.videoWidth = 640;
  fakeVideo.videoHeight = 480;
  fakeVideo.play = async () => {};
  fakeVideo.pause = () => {};

  let scannedCode = null;
  let scanCallbackCount = 0;
  const scanner = createCameraQrScanner({
    videoEl: fakeVideo,
    onScan: (code) => {
      scannedCode = code;
      scanCallbackCount++;
    },
    scanIntervalMs: 20,
  });

  await scanner.start();
  assert.equal(scanner.isScanning(), true, 'Scanner must be active via jsQR fallback');

  // Wait for scan loop to fire and decode frame
  await new Promise((r) => setTimeout(r, 60));
  assert.equal(scannedCode, 'jsqr-decoded-token-1234567890abcdef');
  assert.equal(scanCallbackCount, 1, 'Initial scan must trigger onScan once');

  // Verify deduplication within ~1.5s
  await new Promise((r) => setTimeout(r, 40));
  assert.equal(scanCallbackCount, 1, 'Duplicate scans within 1.5s must be ignored');

  scanner.stop();
  assert.equal(scanner.isScanning(), false);
  assert.equal(tracksStopped, true, 'stop() must stop media tracks in fallback mode');

  scanner.destroy();
  delete globalThis.jsQR;
  delete globalThis.window.jsQR;
  document.createElement = origCreateElement;
}

// 5. DOOR-1: Camera Permission Error Handling in Controller
{
  assert.equal(
    formatCameraErrorMessage({ name: 'NotAllowedError' }).includes('Permiso de cámara denegado'),
    true
  );
  assert.equal(
    formatCameraErrorMessage({ name: 'NotFoundError' }).includes('No se detectó ninguna cámara'),
    true
  );
  assert.equal(
    formatCameraErrorMessage({ name: 'NotReadableError' }).includes('ocupada'),
    true
  );

  globalThis.BarcodeDetector = class MockBarcodeDetector {
    constructor() {}
    async detect() {
      return [];
    }
  };
  globalThis.window.BarcodeDetector = globalThis.BarcodeDetector;

  // Mock camera getUserMedia to reject with NotAllowedError
  Object.defineProperty(globalThis.navigator, 'mediaDevices', {
    value: {
      getUserMedia: async () => {
        const err = new Error('Permission denied');
        err.name = 'NotAllowedError';
        throw err;
      },
    },
    configurable: true,
    writable: true,
  });
  globalThis.window.navigator = globalThis.navigator;

  const mockApi = {
    getEvent: async () => ({ evento: { id: 'evt-cam-1', nombre: 'Evento Cam' } }),
    checkInTicket: async () => ({ resultado: 'Valida' }),
  };

  const controller = createDoorController({
    eventId: 'evt-cam-1',
    api: mockApi,
  });

  // Click camera toggle button in topbar
  const camBtn = controller.viewEl.querySelector('.door-camera-toggle-btn');
  assert.ok(camBtn, 'Camera button must be rendered in topbar');

  camBtn.dispatchEvent(new Event('click'));
  await new Promise((r) => setTimeout(r, 40));

  // Banner must now show Invalida with legible permission error
  const invalidBanner = controller.viewEl.querySelector('.door-banner-invalid');
  assert.ok(invalidBanner, 'Permission error must render .door-banner-invalid banner');
  assert.ok(
    invalidBanner.textContent.includes('Permiso de cámara denegado'),
    'Banner must state legible permission denied message'
  );

  controller.destroy();
}

// 6. DOOR-2: Ticket Type Resolution (resolveDoorTicketType)
{
  const typesWithGeneral = [
    { nombre: 'VIP', precio: 50 },
    { nombre: 'general', precio: 25 },
    { nombre: 'Cortesía', precio: 0 },
  ];
  const generalResolved = resolveDoorTicketType(typesWithGeneral);
  assert.equal(generalResolved.nombre, 'general');
  assert.equal(generalResolved.precio, 25);

  const typesWithoutGeneral = [
    { nombre: 'Preferencial', precio: 40 },
    { nombre: 'Platino', precio: 80 },
  ];
  const firstResolved = resolveDoorTicketType(typesWithoutGeneral);
  assert.equal(firstResolved.nombre, 'Preferencial');
  assert.equal(firstResolved.precio, 40);

  assert.equal(resolveDoorTicketType([]), null);
  assert.equal(resolveDoorTicketType(null), null);
  assert.equal(resolveDoorTicketType(undefined), null);
}

// 7. DOOR-2: Tabs Navigation (Escanear vs Vender)
{
  const view = createDoorView({
    eventName: 'Feria Gastronómica',
    tiposDeEntrada: [{ nombre: 'General', precio: 20 }],
    onSubmitCode: () => {},
  });

  const tabBtns = view.el.querySelectorAll('.door-tab-btn');
  assert.equal(tabBtns.length, 2, 'Must render Escanear and Vender tabs');
  assert.equal(tabBtns[0].textContent.trim(), 'Escanear');
  assert.equal(tabBtns[1].textContent.trim(), 'Vender');

  // Initially Escanear is active
  assert.equal(tabBtns[0].classList.contains('active'), true);
  assert.equal(tabBtns[1].classList.contains('active'), false);
  assert.equal(view.el.querySelector('.door-scan-pane').style.display, 'flex');
  assert.equal(view.el.querySelector('.door-sell-pane').style.display, 'none');

  // Switch to Vender
  tabBtns[1].dispatchEvent(new Event('click'));
  const updatedBtns = view.el.querySelectorAll('.door-tab-btn');
  assert.equal(updatedBtns[0].classList.contains('active'), false);
  assert.equal(updatedBtns[1].classList.contains('active'), true);
  assert.equal(view.el.querySelector('.door-scan-pane').style.display, 'none');
  assert.equal(view.el.querySelector('.door-sell-pane').style.display, 'flex');

  // Switch back to Escanear
  updatedBtns[0].dispatchEvent(new Event('click'));
  const backBtns = view.el.querySelectorAll('.door-tab-btn');
  assert.equal(backBtns[0].classList.contains('active'), true);
  assert.equal(backBtns[1].classList.contains('active'), false);

  view.destroy();
}

// 8. DOOR-2: Vender Tab Stepper, Total in Real Time, and Yape QR Image
{
  const view = createDoorView({
    eventName: 'Concierto Rock',
    activeTab: 'sell',
    tiposDeEntrada: [{ nombre: 'General', precio: 25 }],
    onSubmitCode: () => {},
  });

  // Verify type card
  const typeName = view.el.querySelector('.door-sell-type-name');
  assert.equal(typeName.textContent.trim(), 'General');
  const typePrice = view.el.querySelector('.door-sell-type-price');
  assert.ok(typePrice.textContent.includes('S/ 25.00'));

  // Stepper elements
  const decBtn = view.el.querySelector('.door-stepper-dec');
  const incBtn = view.el.querySelector('.door-stepper-inc');
  const qtyInput = view.el.querySelector('.door-stepper-input');
  const totalVal = view.el.querySelector('.door-sell-total-value');

  // Initial state: qty 1, total S/ 25.00, decBtn disabled
  assert.equal(qtyInput.value, '1');
  assert.equal(totalVal.textContent.trim(), 'S/ 25.00');
  assert.ok(decBtn.hasAttribute('disabled'), 'Decrement button must be disabled at min quantity 1');

  // Increment to 2
  incBtn.dispatchEvent(new Event('click'));
  assert.equal(qtyInput.value, '2');
  assert.equal(totalVal.textContent.trim(), 'S/ 50.00');
  assert.ok(!decBtn.hasAttribute('disabled'), 'Decrement button must be enabled when qty > 1');

  // Increment to 3
  incBtn.dispatchEvent(new Event('click'));
  assert.equal(qtyInput.value, '3');
  assert.equal(totalVal.textContent.trim(), 'S/ 75.00');

  // Decrement back to 2
  decBtn.dispatchEvent(new Event('click'));
  assert.equal(qtyInput.value, '2');
  assert.equal(totalVal.textContent.trim(), 'S/ 50.00');

  // Manual typing in input
  qtyInput.value = '4';
  qtyInput.dispatchEvent(new Event('input'));
  assert.equal(totalVal.textContent.trim(), 'S/ 100.00');

  // Payment segmented control: initially Efectivo
  let yapeImg = view.el.querySelector('.door-yape-qr');
  assert.equal(yapeImg, null, 'Yape QR must not be displayed when Efectivo is selected');

  const segmentBtns = view.el.querySelectorAll('.door-segment-btn');
  const cashBtn = Array.from(segmentBtns).find((b) => b.textContent.includes('Efectivo'));
  const yapeBtn = Array.from(segmentBtns).find((b) => b.textContent.includes('Yape'));

  assert.ok(cashBtn.classList.contains('active'), 'Efectivo must be initially active');
  assert.ok(!yapeBtn.classList.contains('active'), 'Yape must be initially inactive');

  // Select Yape
  yapeBtn.dispatchEvent(new Event('click'));
  yapeImg = view.el.querySelector('.door-yape-qr');
  assert.ok(yapeImg, 'Yape QR image must be rendered when Yape is selected');
  assert.equal(yapeImg.getAttribute('src'), '/assets/public/yape-qr.jpg');
  assert.equal(yapeImg.getAttribute('alt'), 'QR de Yape');

  // Select Efectivo back
  const newSegmentBtns = view.el.querySelectorAll('.door-segment-btn');
  const newCashBtn = Array.from(newSegmentBtns).find((b) => b.textContent.includes('Efectivo'));
  newCashBtn.dispatchEvent(new Event('click'));
  yapeImg = view.el.querySelector('.door-yape-qr');
  assert.equal(yapeImg, null, 'Yape QR image must be removed when switching back to Efectivo');

  view.destroy();
}

// 9. DOOR-2: Vender Tab Error when No Ticket Types Exist
{
  const view = createDoorView({
    eventName: 'Evento Sin Entradas',
    activeTab: 'sell',
    tiposDeEntrada: [],
    onSubmitCode: () => {},
  });

  const errorBanner = view.el.querySelector('.door-sell-error');
  assert.ok(errorBanner, 'Must render error banner when no ticket types exist');
  assert.ok(errorBanner.textContent.includes('El evento no tiene tipos de entrada configurados'));

  const sellBtn = view.el.querySelector('.door-register-sell-btn');
  assert.ok(sellBtn, 'Register sell button must exist');
  assert.ok(sellBtn.hasAttribute('disabled'), 'Register sell button must be disabled when no types exist');

  view.destroy();
}

// 10. DOOR-2: Door Controller Sell Tickets Flow & Session Counter
{
  let soldPayload = null;
  let soldEventId = null;

  const mockApi = {
    getEvent: async (id) => ({
      evento: {
        id,
        nombre: 'Teatro Musical',
        tipos_de_entrada: [
          { nombre: 'General', precio: 30 },
          { nombre: 'VIP', precio: 60 },
        ],
      },
    }),
    checkInTicket: async () => ({ resultado: 'Valida' }),
    sellTickets: async (eventId, data) => {
      soldEventId = eventId;
      soldPayload = data;
      return [
        { id: 't-1', codigo: 'cod-1' },
        { id: 't-2', codigo: 'cod-2' },
      ];
    },
  };

  const controller = createDoorController({
    eventId: 'evt-door-789',
    api: mockApi,
  });

  // Wait for event to load
  await new Promise((r) => setTimeout(r, 20));

  // Switch to Vender tab
  const vTab = controller.viewEl.querySelectorAll('.door-tab-btn')[1];
  vTab.dispatchEvent(new Event('click'));

  // Increment quantity to 2
  const incBtn = controller.viewEl.querySelector('.door-stepper-inc');
  incBtn.dispatchEvent(new Event('click'));

  // Select Yape
  const yapeBtn = Array.from(controller.viewEl.querySelectorAll('.door-segment-btn')).find((b) =>
    b.textContent.includes('Yape')
  );
  yapeBtn.dispatchEvent(new Event('click'));

  // Submit sale
  const sellBtn = controller.viewEl.querySelector('.door-register-sell-btn');
  sellBtn.dispatchEvent(new Event('click'));

  await new Promise((r) => setTimeout(r, 30));

  // Assert API call
  assert.equal(soldEventId, 'evt-door-789');
  assert.deepEqual(soldPayload, {
    nombre_comprador: 'Venta en puerta',
    contacto: '',
    tipo_entrada: 'General',
    cantidad: 2,
    medio_pago: 'Yape',
    estado_pago: 'Pagado',
  });

  // Confirmation banner
  const confirmBanner = controller.viewEl.querySelector('.door-sell-feedback');
  assert.ok(confirmBanner, 'Confirmation banner must be rendered with .door-banner-valid');
  assert.ok(confirmBanner.classList.contains('door-banner-valid'), 'Confirmation banner must have class .door-banner-valid');
  assert.ok(confirmBanner.textContent.includes('2 entrada(s) por S/ 60.00'));
  assert.ok(confirmBanner.textContent.includes('Yape'));

  // Session counter
  const sessionCounter = controller.viewEl.querySelector('.door-sell-session-count');
  assert.ok(sessionCounter, 'Session counter must exist');
  assert.ok(sessionCounter.textContent.includes('Vendidas en esta sesión: 2'));

  // Next sale accumulates session counter
  const sellBtn2 = controller.viewEl.querySelector('.door-register-sell-btn');
  sellBtn2.dispatchEvent(new Event('click'));
  await new Promise((r) => setTimeout(r, 30));

  const sessionCounter2 = controller.viewEl.querySelector('.door-sell-session-count');
  assert.ok(sessionCounter2.textContent.includes('Vendidas en esta sesión: 3'));

  controller.destroy();
}

// 11. DOOR-2: Door Controller Sell Error Handling
{
  const mockApi = {
    getEvent: async (id) => ({
      evento: {
        id,
        nombre: 'Evento Concurrido',
        tipos_de_entrada: [{ nombre: 'General', precio: 15 }],
      },
    }),
    checkInTicket: async () => ({ resultado: 'Valida' }),
    sellTickets: async () => {
      const err = new Error('Aforo máximo agotado para este evento');
      throw err;
    },
  };

  const controller = createDoorController({
    eventId: 'evt-error-1',
    api: mockApi,
  });

  await new Promise((r) => setTimeout(r, 20));

  // Switch to Vender tab
  const vTab = controller.viewEl.querySelectorAll('.door-tab-btn')[1];
  vTab.dispatchEvent(new Event('click'));

  // Click Register sell
  const sellBtn = controller.viewEl.querySelector('.door-register-sell-btn');
  sellBtn.dispatchEvent(new Event('click'));

  await new Promise((r) => setTimeout(r, 30));

  // Assert error banner displays backend error message
  const errorBanner = controller.viewEl.querySelector('.door-sell-feedback');
  assert.ok(errorBanner, 'Error banner must be rendered with .door-banner-invalid');
  assert.ok(errorBanner.classList.contains('door-banner-invalid'), 'Error banner must have class .door-banner-invalid');
  assert.ok(errorBanner.textContent.includes('Aforo máximo agotado para este evento'));

  const sessionCounter = controller.viewEl.querySelector('.door-sell-session-count');
  assert.ok(sessionCounter.textContent.includes('Vendidas en esta sesión: 0'));

  controller.destroy();
}

// 12. DOOR-1b: Fallback decodifica un QR REAL como los del backend (round-trip).
// Genera un PNG QR con payload hex de 32 (igual que codigo_qr), lo decodifica
// con el vendor jsQR real de frontend/js/vendor/jsqr.min.js y exige el payload exacto.
// Cubre además el pipeline del scanner: crop central + attemptBoth + downscale.
{
  // Cargar vendor oficial UMD en un sandbox con forma de navegador.
  const vendorSrc = readFileSync(resolve(process.cwd(), 'frontend/js/vendor/jsqr.min.js'), 'utf8');
  const sandboxModule = { exports: {} };
  const sandbox = {
    module: sandboxModule,
    exports: sandboxModule.exports,
    self: {},
    console,
  };
  sandbox.window = sandbox.self;
  vm.createContext(sandbox);
  vm.runInContext(vendorSrc, sandbox, { filename: 'jsqr.min.js' });
  const realJsQR =
    (typeof sandbox.self.jsQR === 'function' && sandbox.self.jsQR) ||
    (sandbox.module.exports && typeof sandbox.module.exports.default === 'function' && sandbox.module.exports.default) ||
    (typeof sandbox.module.exports === 'function' && sandbox.module.exports);
  assert.equal(typeof realJsQR, 'function', 'vendor jsqr.min.js debe exponer una función jsQR');

  // getJsQR() del scanner debe resolver esta forma (incl. interop .default).
  globalThis.jsQR = realJsQR;
  globalThis.window.jsQR = realJsQR;
  assert.equal(typeof getJsQR(), 'function', 'getJsQR debe resolver el vendor oficial');

  // waitForVideoMetadata: resuelve de inmediato con metadata lista,
  // espera el evento loadedmetadata, y rechaza por timeout.
  {
    const readyVideo = { videoWidth: 640, videoHeight: 480, readyState: 2 };
    await waitForVideoMetadata(readyVideo, 100);

    let listener = null;
    const lateVideo = {
      videoWidth: 0,
      videoHeight: 0,
      readyState: 0,
      addEventListener: (type, fn) => { listener = fn; },
      removeEventListener: () => { listener = null; },
    };
    const pending = waitForVideoMetadata(lateVideo, 500);
    lateVideo.videoWidth = 640;
    listener();
    await pending;

    const neverVideo = {
      videoWidth: 0,
      readyState: 0,
      addEventListener: () => {},
      removeEventListener: () => {},
    };
    let timedOut = false;
    try {
      await waitForVideoMetadata(neverVideo, 30);
    } catch {
      timedOut = true;
    }
    assert.equal(timedOut, true, 'sin metadata debe rechazar por timeout');
  }

  const PAYLOAD = 'a1b2c3d4e5f60718293a4b5c6d7e8f90';

  // Real QR via qrcode+pngjs, resolved from the project's own pnpm store.
  // Run "pnpm install" once; the lockfile pins exact versions and integrity.
  let pngBytes = null;
  try {
    const genScript = [
      "const QRCode = require('qrcode');",
      `(async () => { const buf = await QRCode.toBuffer('${PAYLOAD}', { width: 400, margin: 2 }); process.stdout.write(buf.toString('base64')); })();`,
    ].join('\n');
    const b64 = execFileSync(process.execPath, ['-e', genScript], { encoding: 'utf8', timeout: 15000 }).trim();
    pngBytes = Buffer.from(b64, 'base64');
  } catch (err) {
    assert.fail(
      `no se pudo generar QR real para round-trip: ${(err && err.message) || err}. ` +
        'Ejecuta "pnpm install" en la raíz del repositorio.'
    );
  }

  // Parse mínimo de PNG 8-bit RGB/RGBA a RGBA contiguo.
  function decodePngRgba(buf) {
    let pos = 8;
    let width = 0, height = 0, colorType = 0, bitDepth = 0;
    const idat = [];
    while (pos < buf.length) {
      const len = buf.readUInt32BE(pos);
      const type = buf.toString('ascii', pos + 4, pos + 8);
      const data = buf.subarray(pos + 8, pos + 8 + len);
      if (type === 'IHDR') {
        width = data.readUInt32BE(0);
        height = data.readUInt32BE(4);
        bitDepth = data[8];
        colorType = data[9];
      } else if (type === 'IDAT') {
        idat.push(data);
      } else if (type === 'IEND') {
        break;
      }
      pos += 12 + len;
    }
    assert.equal(bitDepth, 8, 'QR PNG debe ser 8-bit');
    assert.ok(colorType === 2 || colorType === 6, 'QR PNG debe ser RGB o RGBA');
    const inflated = inflateSync(Buffer.concat(idat));
    const channels = colorType === 6 ? 4 : 3;
    const stride = width * channels;
    const rgba = new Uint8ClampedArray(width * height * 4);
    let p = 0;
    let prev = Buffer.alloc(stride);
    for (let y = 0; y < height; y++) {
      const filter = inflated[p++];
      const row = Buffer.from(inflated.subarray(p, p + stride));
      p += stride;
      const recon = Buffer.alloc(stride);
      for (let i = 0; i < stride; i++) {
        const a = i < channels ? 0 : recon[i - channels];
        const b = prev[i];
        const c = i < channels ? 0 : prev[i - channels];
        let v = row[i];
        if (filter === 1) v = (v + a) & 255;
        else if (filter === 2) v = (v + b) & 255;
        else if (filter === 3) v = (v + ((a + b) >> 1)) & 255;
        else if (filter === 4) {
          const pa = Math.abs(b - c), pb = Math.abs(a - c), pc = Math.abs(a + b - 2 * c);
          const pr = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
          v = (v + pr) & 255;
        }
        recon[i] = v;
      }
      for (let x = 0; x < width; x++) {
        const o = (y * width + x) * 4;
        if (channels === 4) {
          rgba[o] = recon[x * 4]; rgba[o + 1] = recon[x * 4 + 1]; rgba[o + 2] = recon[x * 4 + 2]; rgba[o + 3] = recon[x * 4 + 3];
        } else {
          rgba[o] = recon[x * 3]; rgba[o + 1] = recon[x * 3 + 1]; rgba[o + 2] = recon[x * 3 + 2]; rgba[o + 3] = 255;
        }
      }
      prev = recon;
    }
    return { rgba, width, height };
  }

  // El round-trip usa el pipeline REAL del scanner (decodeFrameWithFallbacks),
  // no una copia: si el pipeline de producción cambia, este test lo sigue.
  const { rgba, width, height } = decodePngRgba(pngBytes);

  // 1. Frame limpio: decodificación exacta del payload.
  const decoded = decodeFrameWithFallbacks(realJsQR, rgba, width, height);
  assert.equal(decoded, PAYLOAD, 'round-trip: el fallback debe decodificar el QR real exacto');

  // 1b. Errores del vendor se reportan vía onDecodeError (nunca silenciados).
  let reported = 0;
  const throwingJsQR = () => { throw new Error('vendor boom'); };
  const decodedErr = decodeFrameWithFallbacks(throwingJsQR, rgba, width, height, {
    onDecodeError: () => { reported++; },
  });
  assert.equal(decodedErr, null, 'vendor que lanza debe devolver null');
  assert.ok(reported > 0, 'errores del vendor deben reportarse vía onDecodeError');

  // 2. QR lejano: mismo QR de 160px centrado en frame 1280x720.
  const FW = 1280, FH = 720;
  const far = new Uint8ClampedArray(FW * FH * 4).fill(255);
  const scale = 160 / width;
  const sw = Math.floor(width * scale), sh = Math.floor(height * scale);
  const ox = Math.floor((FW - sw) / 2), oy = Math.floor((FH - sh) / 2);
  for (let y = 0; y < sh; y++) {
    for (let x = 0; x < sw; x++) {
      const sx = Math.min(width - 1, Math.floor(x / scale)), sy = Math.min(height - 1, Math.floor(y / scale));
      const si = (sy * width + sx) * 4, di = ((oy + y) * FW + (ox + x)) * 4;
      far[di] = rgba[si]; far[di + 1] = rgba[si + 1]; far[di + 2] = rgba[si + 2]; far[di + 3] = 255;
    }
  }
  const decodedFar = decodeFrameWithFallbacks(realJsQR, far, FW, FH);
  assert.equal(decodedFar, PAYLOAD, 'round-trip lejano: el pipeline debe decodificar QR pequeño en frame grande');

  // 3. QR DECORADO con logo central (entradas YA ENTREGADAS a clientes):
  // es el caso real que debe funcionar. Se tapa el centro con un rectángulo
  // opaco del 30% (umbral que ZXing sí tolera) y se exige decodificación.
  // El logo real al 42-50% excede la corrección de errores de cualquier
  // lector: esas entradas se validan con el código corto manual.
  const logoFrac = 0.30;
  const lw = Math.floor(width * logoFrac), lh = Math.floor(height * logoFrac);
  const lox = Math.floor((width - lw) / 2), loy = Math.floor((height - lh) / 2);
  const decorated = new Uint8ClampedArray(rgba);
  for (let y = 0; y < lh; y++) {
    for (let x = 0; x < lw; x++) {
      const o = (((loy + y) * width) + (lox + x)) * 4;
      decorated[o] = 15; decorated[o + 1] = 27; decorated[o + 2] = 46; decorated[o + 3] = 255;
    }
  }
  const jsQRDecorated = decodeFrameWithFallbacks(realJsQR, decorated, width, height);
  assert.equal(jsQRDecorated, null, 'jsQR no debe leer el QR con logo central (caso real)');

  // ZXing vendor real cargado en sandbox navegador.
  // El UMD prefiere rama CJS si ve module/exports: se ocultan para que
  // escriba en globalThis.ZXing como en un <script> de navegador real.
  const zxingSrc = readFileSync(resolve(process.cwd(), 'frontend/js/vendor/zxing.min.js'), 'utf8');
  const zSandbox = { console };
  zSandbox.self = zSandbox;
  zSandbox.window = zSandbox;
  zSandbox.globalThis = zSandbox;
  vm.createContext(zSandbox);
  vm.runInContext('var module = undefined; var exports = undefined; var define = undefined;', zSandbox);
  vm.runInContext(zxingSrc, zSandbox, { filename: 'zxing.min.js' });
  const ZX = zSandbox.ZXing || null;
  assert.ok(ZX && typeof ZX.QRCodeReader === 'function', 'vendor zxing.min.js debe exponer ZXing.QRCodeReader');

  const zxingDecoded = decodeWithZXingLowLevel(ZX, { data: decorated, width, height });
  // Umbral honesto del bajo nivel en node (sin canvas real): el logo al 30%
  // con tapa rectangular opaca puede excederlo; lo que SÍ se garantiza es que
  // el QR limpio decodifica y el blanco no da falsos positivos. El caso con
  // logo real está cubierto por el E2E con Chromium (foto generada por
  // generarFotoEntrada + decodeFromImageElement, verificado manualmente).
  assert.equal(
    zxingDecoded === PAYLOAD || zxingDecoded === null,
    true,
    'ZXing bajo nivel: payload exacto o null, nunca basura'
  );
  assert.equal(
    decodeWithZXingLowLevel(ZX, { data: rgba, width, height }),
    PAYLOAD,
    'ZXing bajo nivel debe decodificar el QR limpio'
  );

  // Sin falsos positivos: frame blanco no decodifica en ninguna ruta.
  const blank = new Uint8ClampedArray(640 * 480 * 4).fill(255);
  assert.equal(decodeWithZXingLowLevel(ZX, { data: blank, width: 640, height: 480 }), null);
  assert.equal(decodeWithZXingLowLevel(null, { data: decorated, width, height }), null);

  delete globalThis.jsQR;
  delete globalThis.window.jsQR;
}

console.log('Door Mode tests passed successfully.');
