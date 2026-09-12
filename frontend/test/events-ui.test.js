import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { setupDomEnvironment } from '../../tests/fake-dom.mjs';

setupDomEnvironment();

const topbarModulePath = resolve(process.cwd(), 'frontend/js/components/organisms/app-topbar.js');
const { createAppTopbar } = await import(pathToFileURL(topbarModulePath).href);

const viewsModulePath = resolve(process.cwd(), 'frontend/js/modules/events/views.js');
const { createEventsCatalogView, createEventDetailView, formatEventDateTime } = await import(pathToFileURL(viewsModulePath).href);

const formsModulePath = resolve(process.cwd(), 'frontend/js/modules/events/forms.js');
const {
  createEventFormDialog,
  createExpenseFormDialog,
  createInterestFormDialog,
  createConvertInterestDialog,
  createSellTicketDialog,
} = await import(pathToFileURL(formsModulePath).href);

const routerModulePath = resolve(process.cwd(), 'frontend/js/router/parse-route.js');
const { parseRoute } = await import(pathToFileURL(routerModulePath).href);

const navigateModulePath = resolve(process.cwd(), 'frontend/js/router/navigate.js');
const { toPresentations, toEvents, toEvent } = await import(pathToFileURL(navigateModulePath).href);

console.log('Running Events UI and Integration Tests...');

// 1. App Topbar with Section Selector (Repertorios | Eventos)
{
  let selectedSection = null;
  const topbar = createAppTopbar({
    title: 'Shirley Show',
    activeSection: 'presentations',
    onSectionChange: (sec) => {
      selectedSection = sec;
    },
  });

  const nav = topbar.el.querySelector('.app-topbar-nav');
  assert.ok(nav, 'Topbar must render .app-topbar-nav when activeSection is set');

  const buttons = topbar.el.querySelectorAll('.app-topbar-nav-btn');
  assert.equal(buttons.length, 2, 'Must render exactly 2 section buttons: Repertorios and Eventos');
  assert.equal(buttons[0].textContent, 'Repertorios');
  assert.equal(buttons[1].textContent, 'Eventos');

  assert.equal(buttons[0].getAttribute('aria-current'), 'page');
  assert.ok(buttons[0].getAttribute('class').includes('is-active'));
  assert.equal(buttons[1].getAttribute('aria-current'), null);

  // Click on Eventos
  buttons[1].dispatchEvent(new Event('click'));
  assert.equal(selectedSection, 'events', 'Clicking Eventos button triggers onSectionChange with "events"');

  // Update topbar to events active
  topbar.update({ activeSection: 'events' });
  const updatedButtons = topbar.el.querySelectorAll('.app-topbar-nav-btn');
  assert.equal(updatedButtons[1].getAttribute('aria-current'), 'page');
  assert.ok(updatedButtons[1].getAttribute('class').includes('is-active'));
}

// 2. Events Catalog View
{
  const mockEvents = [
    {
      id: 'evt-1',
      nombre: 'Noche Criolla Barranco',
      lugar: 'La Noche',
      fecha_hora: '2026-10-15T21:00:00Z',
      finalizado: 0,
      imagen: 'evt1_poster.jpg',
      tipos_de_entrada: [
        { nombre: 'General', precio: 30, capacidad: 50 },
        { nombre: 'VIP', precio: 60, capacidad: 20 },
      ],
    },
    {
      id: 'evt-2',
      nombre: 'Concierto Aniversario',
      lugar: 'Teatro Municipal',
      fecha_hora: '2026-05-10T20:00:00Z',
      finalizado: 1,
      imagen: null,
      tipos_de_entrada: [
        { nombre: 'Preferencial', precio: 40, capacidad: 80 },
      ],
    },
  ];

  let selectedEventId = null;
  let editedEvent = null;
  let deletedEvent = null;

  const catalogView = createEventsCatalogView({
    events: mockEvents,
    isLoading: false,
    errorMessage: null,
    onCreateEvent: () => {},
    onSelectEvent: (id) => { selectedEventId = id; },
    onEditEvent: (evt) => { editedEvent = evt; },
    onDeleteEvent: (evt) => { deletedEvent = evt; },
  });

  // Verify sections
  const cards = catalogView.el.querySelectorAll('.events-card');
  assert.equal(cards.length, 2, 'Must render two event cards');

  // Active event card
  assert.ok(!cards[0].getAttribute('class').includes('is-finalized'));
  assert.ok(cards[0].textContent.includes('Noche Criolla Barranco'));
  assert.ok(cards[0].textContent.includes('La Noche'));
  assert.ok(cards[0].textContent.includes('General: S/ 30.00'));
  assert.ok(cards[0].textContent.includes('Activo'));

  // Finalized event card
  assert.ok(cards[1].getAttribute('class').includes('is-finalized'));
  assert.ok(cards[1].textContent.includes('Finalizado'));

  // Click card triggers onSelectEvent
  cards[0].dispatchEvent(new Event('click'));
  assert.equal(selectedEventId, 'evt-1');
}

// 3. Event Detail View with Tabs & 7 Metrics
{
  const mockEvent = {
    id: 'evt-1',
    nombre: 'Noche Criolla Barranco',
    lugar: 'La Noche',
    fecha_hora: '2026-10-15T21:00:00Z',
    finalizado: 0,
    imagen: 'evt1_poster.jpg',
    tipos_de_entrada: [{ nombre: 'General', precio: 30, capacidad: 50 }],
  };

  const mockSummary = {
    cobrado: 300.0,
    pendiente: 120.0,
    total_proyectado: 420.0,
    gastos: 180.0,
    ganancia_neta: 120.0,
    capacidad_total: 50,
    entradas_emitidas: 16,
    aforo_proyectado: 19,
    por_medio_pago: {
      Yape: { cobrado: 300.0, pendiente: 0.0, cantidad: 10 },
      Efectivo: { cobrado: 0.0, pendiente: 120.0, cantidad: 4 },
      Cortesía: { cobrado: 0.0, pendiente: 0.0, cantidad: 2 },
    },
    gastos_por_categoria: {
      'Local/Alquiler': 100.0,
      'Sonido/Luces': 80.0,
    },
  };

  const mockTickets = [
    {
      id: 'tix-1',
      evento_id: 'evt-1',
      nombre_comprador: 'Valeria Díaz',
      contacto: '999111222',
      tipo_entrada: 'General',
      precio: 30.0,
      medio_pago: 'Yape',
      estado_pago: 'Pagado',
      estado_checkin: 'No ingresó',
    },
    {
      id: 'tix-2',
      evento_id: 'evt-1',
      nombre_comprador: 'Pedro Gómez',
      contacto: '999333444',
      tipo_entrada: 'General',
      precio: 30.0,
      medio_pago: 'Efectivo',
      estado_pago: 'Pendiente',
      estado_checkin: 'No ingresó',
    },
  ];

  const mockInterests = [
    {
      id: 'int-1',
      evento_id: 'evt-1',
      nombre: 'Mariana Rios',
      contacto: '987654321',
      canal: 'Instagram',
      promotora: 'Shirley',
      personas: 3,
      estado: 'Pendiente',
      notas: 'Mesa cerca al escenario',
    },
  ];

  const mockExpenses = [
    {
      id: 'exp-1',
      evento_id: 'evt-1',
      categoria: 'Local/Alquiler',
      monto: 100.0,
      descripcion: 'Adelanto de local',
      fecha: '2026-10-01',
    },
    {
      id: 'exp-2',
      evento_id: 'evt-1',
      categoria: 'Sonido/Luces',
      monto: 80.0,
      descripcion: 'Consola de audio',
      fecha: '2026-10-02',
    },
  ];

  let currentTab = 'resumen';
  let doorOpened = false;

  const detailView = createEventDetailView({
    event: mockEvent,
    summary: mockSummary,
    tickets: mockTickets,
    interests: mockInterests,
    expenses: mockExpenses,
    activeTab: currentTab,
    onTabChange: (tab) => {
      currentTab = tab;
      detailView.update({ activeTab: tab });
    },
    onOpenDoor: () => { doorOpened = true; },
    onFinalize: () => {},
    onReopen: () => {},
    onUploadPoster: () => {},
    onEditEvent: () => {},
    onDeleteEvent: () => {},
    onSellTickets: () => {},
    onConfirmPayment: () => {},
    onDeleteTicket: () => {},
    onCreateInterest: () => {},
    onEditInterest: () => {},
    onDeleteInterest: () => {},
    onConvertInterest: () => {},
    onCreateExpense: () => {},
    onEditExpense: () => {},
    onDeleteExpense: () => {},
  });

  // Check Hero
  assert.ok(detailView.el.textContent.includes('Noche Criolla Barranco'));
  assert.ok(detailView.el.textContent.includes('La Noche'));

  // Check 7 Metrics in Resumen
  const metricsValues = detailView.el.querySelectorAll('.events-metric-value');
  assert.equal(metricsValues.length, 7, 'Must render exactly 7 metric cards');
  assert.equal(metricsValues[0].textContent, 'S/ 300.00', 'Cobrado');
  assert.equal(metricsValues[1].textContent, 'S/ 120.00', 'Pendiente');
  assert.equal(metricsValues[2].textContent, 'S/ 420.00', 'Total Proyectado');
  assert.equal(metricsValues[3].textContent, 'S/ 180.00', 'Gastos');
  assert.equal(metricsValues[4].textContent, 'S/ 120.00', 'Ganancia Neta');
  assert.equal(metricsValues[5].textContent, '16 / 50', 'Capacidad / Emitidas');
  assert.equal(metricsValues[6].textContent, '19', 'Aforo Proyectado');

  // Test overcapacity warning
  detailView.update({
    summary: { ...mockSummary, capacidad_total: 15, aforo_proyectado: 20 },
  });
  const alertBanner = detailView.el.querySelector('.events-alert-banner');
  assert.ok(alertBanner, 'Must render .events-alert-banner when aforo_proyectado > capacidad_total');

  // Switch to Entradas Tab
  const tabButtons = detailView.el.querySelectorAll('.events-tab-btn');
  assert.equal(tabButtons.length, 4);
  tabButtons[1].dispatchEvent(new Event('click'));
  assert.equal(currentTab, 'entradas');
  assert.ok(detailView.el.textContent.includes('Valeria Díaz'));
  assert.ok(detailView.el.textContent.includes('Pedro Gómez'));

  // Switch to Interesados Tab
  tabButtons[2].dispatchEvent(new Event('click'));
  assert.equal(currentTab, 'interesados');
  assert.ok(detailView.el.textContent.includes('Mariana Rios'));
  assert.ok(detailView.el.textContent.includes('3 pers.'));

  // Switch to Gastos Tab
  tabButtons[3].dispatchEvent(new Event('click'));
  assert.equal(currentTab, 'gastos');
  assert.ok(detailView.el.textContent.includes('Adelanto de local'));
  assert.ok(detailView.el.textContent.includes('Consola de audio'));
}

// 4. Modal Dialogs creation and destruction
{
  const eventDialog = createEventFormDialog({
    onSave: () => {},
    onCancel: () => {},
  });
  assert.ok(eventDialog.el.querySelector('.events-dialog'));
  eventDialog.destroy();

  const expenseDialog = createExpenseFormDialog({
    onSave: () => {},
    onCancel: () => {},
  });
  assert.ok(expenseDialog.el.querySelector('.events-dialog'));
  expenseDialog.destroy();

  const interestDialog = createInterestFormDialog({
    onSave: () => {},
    onCancel: () => {},
  });
  assert.ok(interestDialog.el.querySelector('.events-dialog'));
  interestDialog.destroy();

  const sellDialog = createSellTicketDialog({
    ticketTypes: [{ nombre: 'General', precio: 30, capacidad: 50 }],
    onSell: () => {},
    onCancel: () => {},
  });
  assert.ok(sellDialog.el.querySelector('.events-dialog'));
  sellDialog.destroy();

  const convertDialog = createConvertInterestDialog({
    interest: { nombre: 'Juan', contacto: '999', personas: 2 },
    ticketTypes: [{ nombre: 'General', precio: 30, capacidad: 50 }],
    onConvert: () => {},
    onCancel: () => {},
  });
  assert.ok(convertDialog.el.querySelector('.events-dialog'));
  convertDialog.destroy();
}

// 5. Default Route & Navigation Transition Tests
{
  // Clean / default route always resolves to presentations
  assert.equal(parseRoute('').view, 'presentations');
  assert.equal(parseRoute('#/').view, 'presentations');
  assert.equal(parseRoute('#/presentations').view, 'presentations');

  // Transition to #/eventos resolves to events catalog
  assert.equal(parseRoute('#/eventos').view, 'events');

  // Transition to #/eventos/:id resolves to event-detail
  const detailRoute = parseRoute('#/eventos/evt-cena-show');
  assert.equal(detailRoute.view, 'event-detail');
  assert.equal(detailRoute.params.id, 'evt-cena-show');

  // Transition back to #/presentations
  assert.equal(parseRoute('#/presentations').view, 'presentations');
}

console.log('All Events UI and Integration Tests passed cleanly!');
