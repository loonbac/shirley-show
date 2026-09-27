import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { setupDomEnvironment } from '../../tests/fake-dom.mjs';

setupDomEnvironment();

const scannerModulePath = resolve(process.cwd(), 'frontend/js/modules/events/camera-qr-scanner.js');
const {
  isBarcodeDetectorSupported,
  isCameraSupported,
  getJsQR,
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

console.log('Door Mode tests passed successfully.');
