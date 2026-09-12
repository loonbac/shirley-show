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

const controllersModulePath = resolve(process.cwd(), 'frontend/js/modules/events/controllers.js');
const { createEventsCatalogController, createEventDetailController } = await import(pathToFileURL(controllersModulePath).href);

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

  // Test Venta Rápida with single ticket type (preselected) and multiple ticket types (not preselected)
  {
    const singleTypeDialog = createSellTicketDialog({
      ticketTypes: [{ nombre: 'General', precio: 30, capacidad: 50 }],
      onSell: () => {},
      onCancel: () => {},
    });
    const typeSelect = singleTypeDialog.el.querySelector('select');
    assert.equal(typeSelect.value, 'General', 'Single ticket type must be preselected');
    singleTypeDialog.destroy();

    const multiTypeDialog = createSellTicketDialog({
      ticketTypes: [
        { nombre: 'General', precio: 30, capacidad: 50 },
        { nombre: 'VIP', precio: 60, capacidad: 20 },
      ],
      onSell: () => {},
      onCancel: () => {},
    });
    const multiSelect = multiTypeDialog.el.querySelector('select');
    assert.equal(multiSelect.value, '', 'Multiple ticket types must NOT be preselected');
    const multiInputs = multiTypeDialog.el.querySelectorAll('input');
    multiInputs[0].value = 'Comprador sin teléfono';
    multiInputs[1].value = '';
    multiTypeDialog.el.querySelector('form').dispatchEvent(new Event('submit'));
    await new Promise((r) => setTimeout(r, 0));
    assert.equal(
      multiTypeDialog.el.querySelector('.events-form-error').textContent,
      'Selecciona un tipo de entrada.',
      'Validation must identify the missing ticket type without blaming the optional phone'
    );
    multiTypeDialog.destroy();
  }

  // Test Venta Rápida submission and issued tickets result cards with WhatsApp links
  {
    let soldData = null;
    const fakeIssuedTickets = [
      {
        id: 't-1',
        tipo_entrada: 'General',
        precio: 30,
        nombre_comprador: 'Ana Silva',
        contacto: '987654321',
        scan_token: 'tok_ana_1',
        medio_pago: 'Yape',
        estado_pago: 'Pagado',
      },
      {
        id: 't-2',
        tipo_entrada: 'General',
        precio: 30,
        nombre_comprador: 'Ana Silva',
        contacto: '987654321',
        scan_token: 'tok_ana_2',
        medio_pago: 'Yape',
        estado_pago: 'Pagado',
      },
    ];

    const quickSale = createSellTicketDialog({
      eventName: 'Noche de Gala',
      ticketTypes: [{ nombre: 'General', precio: 30, capacidad: 50 }],
      onSell: async (data) => {
        soldData = data;
        return fakeIssuedTickets;
      },
      onCancel: () => {},
    });

    const form = quickSale.el.querySelector('form');
    const inputs = quickSale.el.querySelectorAll('input');
    inputs[0].value = 'Ana Silva';
    assert.equal(inputs[1].getAttribute('required'), null, 'Phone must be optional for a quick ticket sale');
    inputs[1].value = '';
    inputs[2].value = '2';

    form.dispatchEvent(new Event('submit'));
    await new Promise((r) => setTimeout(r, 10));

    assert.equal(soldData.nombre_comprador, 'Ana Silva');
    assert.equal(soldData.contacto, '', 'An empty phone must be sent to the API');
    assert.equal(soldData.cantidad, 2);

    const cardsList = quickSale.el.querySelector('.events-quick-tickets-list');
    assert.ok(cardsList, 'Must render .events-quick-tickets-list after successful sale');
    const cards = quickSale.el.querySelectorAll('.events-quick-ticket-card');
    assert.equal(cards.length, 2, 'Must render 2 ticket cards');

    const waLink = cards[0].querySelector('.events-quick-ticket-wa-btn');
    assert.ok(waLink, 'Card must have WhatsApp sharing link');
    assert.ok(waLink.getAttribute('href').startsWith('https://wa.me/51987654321?text='), 'WhatsApp link targets cleaned phone with 51');

    quickSale.destroy();
  }

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

// 6. Controllers Integration with Real Axum Envelopes ({eventos}, {evento}, {resumen}, {entradas}, {interesados}, {gastos})
{
  const realEvent = {
    id: 'evt-real-1',
    nombre: 'Noche Real Criolla',
    lugar: 'Barranco Bar',
    fecha_hora: '2026-11-20T21:00:00Z',
    finalizado: 0,
    imagen: null,
    tipos_de_entrada: [
      { nombre: 'General', precio: 35.0, capacidad: 60 },
    ],
  };

  const realSummaryEnvelope = {
    resumen: {
      cobrado: 350.0,
      pendiente: 105.0,
      total_proyectado: 455.0,
      gastos: 200.0,
      ganancia_neta: 150.0,
      capacidad_total: 60,
      entradas_emitidas: 13,
      aforo_proyectado: 15,
      desglose_metodos_pago: {
        yape_cobrado: 350.0,
        yape_pendiente: 0.0,
        yape_entradas: 10,
        efectivo_cobrado: 0.0,
        efectivo_pendiente: 105.0,
        efectivo_entradas: 3,
        cortesia_entradas: 0,
      },
      desglose_gastos: {
        local_alquiler: 120.0,
        sonido_luces: 80.0,
        musicos_personal: 0.0,
        publicidad_marketing: 0.0,
        bebidas_catering: 0.0,
        transporte: 0.0,
        otros: 0.0,
      },
    },
  };

  const realTicketsEnvelope = {
    entradas: [
      {
        id: 't-real-1',
        evento_id: 'evt-real-1',
        nombre_comprador: 'Rosa Maria',
        contacto: '988111222',
        tipo_entrada: 'General',
        precio: 35.0,
        medio_pago: 'Yape',
        estado_pago: 'Pagado',
        scan_token: 'st_tok_1',
        ticket_path: '#/ticket/st_tok_1',
        estado_checkin: 'No ingresó',
        hora_ingreso: null,
        qr_svg: '<svg>qr</svg>',
      },
    ],
  };

  const realInterestsEnvelope = {
    interesados: [
      {
        id: 'int-real-1',
        evento_id: 'evt-real-1',
        nombre: 'Carlos Perez',
        contacto: '999111333',
        canal: 'WhatsApp',
        promotora: 'Shirley',
        personas: 2,
        estado: 'Pendiente',
        convertido_at: null,
      },
    ],
  };

  const realExpensesEnvelope = {
    gastos: [
      {
        id: 'exp-real-1',
        evento_id: 'evt-real-1',
        categoria: 'Sonido/Luces',
        monto: 80.0,
        descripcion: 'Microfonia',
        fecha: '2026-11-19',
      },
    ],
  };

  globalThis.fetch = async (url, opts = {}) => {
    const urlStr = String(url);
    const method = (opts.method || 'GET').toUpperCase();

    if (urlStr.includes('/api/eventos?incluir_finalizados=true')) {
      return {
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => ({ eventos: [realEvent] }),
      };
    }

    if (urlStr.endsWith('/api/eventos/evt-real-1/resumen')) {
      return {
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => realSummaryEnvelope,
      };
    }

    if (urlStr.endsWith('/api/eventos/evt-real-1/entradas') && method === 'GET') {
      return {
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => realTicketsEnvelope,
      };
    }

    if (urlStr.endsWith('/api/eventos/evt-real-1/entradas') && method === 'POST') {
      return {
        ok: true,
        status: 201,
        headers: { get: () => 'application/json' },
        json: async () => ({
          entradas: [
            {
              id: 't-issued-1',
              tipo_entrada: 'General',
              precio: 35.0,
              nombre_comprador: 'Nuevo Comprador',
              contacto: '912345678',
              scan_token: 'st_tok_new',
              ticket_path: '#/ticket/st_tok_new',
              medio_pago: 'Yape',
              estado_pago: 'Pagado',
            },
          ],
        }),
      };
    }

    if (urlStr.endsWith('/api/eventos/evt-real-1/interesados')) {
      return {
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => realInterestsEnvelope,
      };
    }

    if (urlStr.endsWith('/api/eventos/evt-real-1/gastos')) {
      return {
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => realExpensesEnvelope,
      };
    }

    if (urlStr.endsWith('/api/eventos/evt-real-1') && method === 'GET') {
      return {
        ok: true,
        status: 200,
        headers: { get: () => 'application/json' },
        json: async () => ({ evento: realEvent }),
      };
    }

    return {
      ok: false,
      status: 404,
      headers: { get: () => 'application/json' },
      json: async () => ({ error: 'not found' }),
    };
  };

  // 6.a Test Events Catalog Controller with { eventos: [...] } envelope
  const catalogCtrl = createEventsCatalogController();
  await new Promise((r) => setTimeout(r, 50));

  const catalogCards = catalogCtrl.viewEl.querySelectorAll('.events-card');
  assert.equal(catalogCards.length, 1, 'Catalog must render 1 card from real { eventos: [...] } envelope');
  assert.ok(catalogCards[0].textContent.includes('Noche Real Criolla'));
  catalogCtrl.destroy();

  // 6.b Test Event Detail Controller with real envelopes
  const detailCtrl = createEventDetailController({ eventId: 'evt-real-1' });
  await new Promise((r) => setTimeout(r, 50));

  assert.ok(detailCtrl.topbarEl.textContent.includes('Noche Real Criolla'), 'Topbar receives event.nombre from real { evento } envelope');

  // Verify 7 metrics rendered from normalized { resumen: ... }
  assert.ok(detailCtrl.viewEl.textContent.includes('S/ 350.00'), 'Cobrado metric rendered from envelope');
  assert.ok(detailCtrl.viewEl.textContent.includes('S/ 105.00'), 'Pendiente metric rendered from envelope');
  assert.ok(detailCtrl.viewEl.textContent.includes('S/ 150.00'), 'Ganancia neta rendered from envelope');

  // Verify breakdown tables populated from desglose_metodos_pago and desglose_gastos
  assert.ok(detailCtrl.viewEl.textContent.includes('Desglose por Medio de Pago'));
  assert.ok(detailCtrl.viewEl.textContent.includes('Desglose de Gastos por Categoría'));
  assert.ok(detailCtrl.viewEl.textContent.includes('Yape'));
  assert.ok(detailCtrl.viewEl.textContent.includes('Sonido/Luces'));

  detailCtrl.destroy();
}

console.log('All Events UI and Integration Tests passed cleanly!');
