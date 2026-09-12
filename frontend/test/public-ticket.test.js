import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { setupDomEnvironment } from '../../tests/fake-dom.mjs';

setupDomEnvironment();

const controllerModulePath = resolve(process.cwd(), 'frontend/js/modules/events/public-ticket-controller.js');
const { createPublicTicketController, createPublicTicketView } = await import(pathToFileURL(controllerModulePath).href);

console.log('Running Public Ticket tests...');

// 1. Paid Ticket: renders event info and vector QR code prominently
{
  const paidTicketData = {
    nombre_evento: 'Gran Concierto Shirley Show',
    fecha_hora: '2026-10-15T21:00:00Z',
    lugar: 'Centro de Convenciones Barranco',
    nombre_comprador: 'Carlos Mendoza',
    tipo_entrada: 'General',
    precio: 35.0,
    estado_pago: 'Pagado',
    estado_checkin: 'No ingresó',
    qr_svg: '<svg id="qr-carlos" viewBox="0 0 100 100"><rect width="100" height="100" fill="#000"/></svg>',
  };

  const view = createPublicTicketView({ ticket: paidTicketData });

  assert.ok(view.el.textContent.includes('Gran Concierto Shirley Show'), 'Renders event name');
  assert.ok(view.el.textContent.includes('Carlos Mendoza'), 'Renders buyer name');
  assert.ok(view.el.textContent.includes('General'), 'Renders ticket type');
  assert.ok(view.el.textContent.includes('Barranco'), 'Renders venue');

  const qrBox = view.el.querySelector('.public-ticket-qr-box');
  assert.ok(qrBox, 'Paid ticket MUST render .public-ticket-qr-box container');
  assert.ok(qrBox.innerHTML.includes('<svg id="qr-carlos"'), 'Paid ticket MUST render the vector qr_svg');

  const pendingBanner = view.el.querySelector('.public-ticket-pending-banner');
  assert.equal(pendingBanner, null, 'Paid ticket MUST NOT render pending banner');
}

// 2. Pending Ticket: renders amber warning banner and MUST NOT render QR container or SVG
{
  const pendingTicketData = {
    nombre_evento: 'Gran Concierto Shirley Show',
    fecha_hora: '2026-10-15T21:00:00Z',
    lugar: 'Centro de Convenciones Barranco',
    nombre_comprador: 'Lucía Vargas',
    tipo_entrada: 'VIP',
    precio: 70.0,
    estado_pago: 'Pendiente',
    estado_checkin: 'No ingresó',
    qr_svg: null,
  };

  const view = createPublicTicketView({ ticket: pendingTicketData });

  assert.ok(view.el.textContent.includes('Lucía Vargas'), 'Renders buyer name');
  assert.ok(view.el.textContent.includes('70.00'), 'Renders pending amount');

  const pendingBanner = view.el.querySelector('.public-ticket-pending-banner');
  assert.ok(pendingBanner, 'Pending ticket MUST render amber warning banner');
  assert.ok(pendingBanner.textContent.toLowerCase().includes('pendiente'), 'Pending banner describes pending status');

  // CRITICAL REQUIREMENT: NO QR container or SVG may be rendered for pending ticket
  const qrBox = view.el.querySelector('.public-ticket-qr-box');
  assert.equal(qrBox, null, 'Pending ticket MUST NOT render .public-ticket-qr-box container');
  assert.equal(view.el.querySelector('svg'), null, 'Pending ticket MUST NOT render any SVG');
}

// 3. Controller lifecycle and loading / error handling
{
  let requestedToken = null;
  const mockApi = {
    getPublicTicket: async (token) => {
      requestedToken = token;
      return {
        nombre_evento: 'Noche Acústica',
        fecha_hora: '2026-11-20T20:00:00Z',
        lugar: 'Miraflores',
        nombre_comprador: 'Valeria Salas',
        tipo_entrada: 'Preferencial',
        precio: 45.0,
        estado_pago: 'Pagado',
        estado_checkin: 'No ingresó',
        qr_svg: '<svg id="qr-valeria"></svg>',
      };
    },
  };

  const controller = createPublicTicketController({
    scanToken: 'tok_valeria_999',
    api: mockApi,
  });

  assert.ok(controller.viewEl, 'Controller provides viewEl');
  assert.ok(controller.headerEl, 'Controller provides minimal headerEl');

  // Wait for async load
  await new Promise((r) => setTimeout(r, 10));

  assert.equal(requestedToken, 'tok_valeria_999', 'Controller calls getPublicTicket with scanToken');
  assert.ok(controller.viewEl.textContent.includes('Valeria Salas'), 'View updated with ticket data');

  controller.destroy();
}

console.log('Public Ticket tests passed successfully.');
