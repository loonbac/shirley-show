import { test, expect } from '@playwright/test';
import { execSync } from 'node:child_process';

test.describe.serial('Shirley Show - Flujos de Usuario Reales E2E', () => {

  test('Flujo 1: Presentaciones - Crear presentación y agregar contenido al repertorio', async ({ page }) => {
    // 1. Ir al catálogo de presentaciones
    await page.goto('/panel#/presentations');
    await expect(page.locator('.welcome-header').first()).toBeVisible();

    // 2. Click en "+ Nueva Presentación"
    const newPresBtn = page.locator('button:has-text("Nueva Presentación")').first();
    await expect(newPresBtn).toBeVisible();
    await newPresBtn.click();

    // 3. Llenar modal de creación
    const dialog = page.locator('.presentation-form-dialog');
    await expect(dialog).toBeVisible();
    await dialog.locator('input[name="title"]').fill('Concierto Criollo 2026');
    await dialog.locator('button[type="submit"]:has-text("Crear")').click();

    // 4. El modal se cierra y la presentación aparece en el catálogo
    await expect(dialog).not.toBeVisible();
    const presCard = page.locator('.presentation-card, [data-presentation-id]').filter({ hasText: 'Concierto Criollo 2026' }).first();
    await expect(presCard).toBeVisible();

    // 5. Click en "Abrir show" para abrir el detalle de la presentación
    await presCard.locator('button:has-text("Abrir show"), .presentation-card-title-link').first().click();
    await expect(page).toHaveURL(/#\/presentations\/[a-f0-9-]+/);
    await expect(page.locator('.app-topbar-title, h1, h2').first()).toContainText('Concierto Criollo 2026');
  });

  test('Flujo 2: Eventos - Crear evento con tipos de entrada y selector de fecha real', async ({ page }) => {
    // 1. Ir a la sección de eventos
    await page.goto('/panel#/eventos');
    await expect(page.locator('.events-catalog-header').first()).toBeVisible();

    // 2. Click en "Nuevo Evento" o "Crear Primer Evento"
    const createBtn = page.locator('button:has-text("Nuevo Evento"), button:has-text("Crear Primer Evento")').first();
    await expect(createBtn).toBeVisible();
    await createBtn.click();

    // 3. Modal de nuevo evento abierto
    const dialog = page.locator('.events-dialog').first();
    await expect(dialog).toBeVisible();
    await expect(dialog.locator('.events-dialog-title')).toContainText('Nuevo Evento');

    // 4. Verificar que el campo fecha_hora es de tipo datetime-local (no texto plano)
    const fechaInput = dialog.locator('input[name="fecha_hora"]');
    await expect(fechaInput).toHaveAttribute('type', 'datetime-local');

    // 5. Llenar datos principales
    await dialog.locator('input[name="nombre"]').fill('Gala de Shirley Show en Barranco');
    await dialog.locator('input[name="lugar"]').fill('La Estación de Barranco');
    await fechaInput.fill('2026-11-28T21:00');

    // 6. Configurar tipo de entrada por defecto (General)
    const generalRow = dialog.locator('.events-type-row').first();
    await generalRow.locator('.events-type-price').fill('40');
    await generalRow.locator('.events-type-cap').fill('80');

    // 7. Agregar un segundo tipo de entrada (VIP)
    const addTypeBtn = dialog.locator('button:has-text("+ Agregar Tipo de Entrada")');
    await addTypeBtn.click();

    const rows = dialog.locator('.events-type-row');
    await expect(rows).toHaveCount(2);
    const vipRow = rows.nth(1);
    await vipRow.locator('.events-type-name').fill('Mesa VIP');
    await vipRow.locator('.events-type-price').fill('75');
    await vipRow.locator('.events-type-cap').fill('20');

    // 8. Enviar formulario
    const submitBtn = dialog.locator('button[type="submit"]');
    await submitBtn.click();

    // 9. El modal debe cerrarse y el evento debe aparecer en el catálogo
    await expect(dialog).not.toBeVisible();
    const eventCard = page.locator('.events-card').filter({ hasText: 'Gala de Shirley Show en Barranco' }).first();
    await expect(eventCard).toBeVisible();
    await expect(eventCard).toContainText('La Estación de Barranco');
    await expect(eventCard).toContainText('General: S/ 40.00');
    await expect(eventCard).toContainText('Mesa VIP: S/ 75.00');
  });

  test('Flujo 3: Detalle de Evento, Venta Rápida de Entradas y Resumen Financiero', async ({ page }) => {
    // 1. Ir a eventos y abrir el evento creado
    await page.goto('/panel#/eventos');
    const eventCard = page.locator('.events-card').filter({ hasText: 'Gala de Shirley Show en Barranco' }).first();
    await expect(eventCard).toBeVisible();
    await eventCard.locator('button:has-text("Ver Detalle")').click();

    // 2. Verificar cabecera y pestañas del evento
    await expect(page).toHaveURL(/#\/eventos\/[a-f0-9-]+/);
    await expect(page.locator('.events-hero-title')).toContainText('Gala de Shirley Show en Barranco');

    const tabs = page.locator('.events-tab-btn');
    await expect(tabs.filter({ hasText: 'Resumen' })).toBeVisible();
    await expect(tabs.filter({ hasText: 'Entradas' })).toBeVisible();
    await expect(tabs.filter({ hasText: 'Interesados' })).toBeVisible();
    await expect(tabs.filter({ hasText: 'Gastos' })).toBeVisible();

    // 3. Ir a la pestaña "Entradas"
    await tabs.filter({ hasText: 'Entradas' }).click();
    const sellBtn = page.locator('button:has-text("Vender Entradas")');
    await expect(sellBtn).toBeVisible();
    await sellBtn.click();

    // 4. Modal de venta rápida
    const sellDialog = page.locator('.events-dialog').first();
    await expect(sellDialog).toBeVisible();
    await expect(sellDialog.locator('.events-dialog-title')).toContainText('Venta Rápida');

    await sellDialog.locator('input[name="nombre_comprador"]').fill('Ana Cecilia Valdivia');
    await sellDialog.locator('input[name="contacto"]').fill('998877665');
    await sellDialog.locator('select.events-select').first().selectOption({ label: 'Mesa VIP (S/ 75.00)' });
    await sellDialog.locator('input[name="cantidad"]').fill('2');

    // Seleccionar medio de pago Yape
    const medioSelect = sellDialog.locator('select.events-select').nth(1);
    await medioSelect.selectOption('Yape');

    // Emitir entradas
    await sellDialog.locator('button[type="submit"]:has-text("Emitir")').click();

    // 5. Debe mostrarse la confirmación con las 2 entradas emitidas
    await expect(sellDialog.locator('.events-dialog-title')).toContainText('Venta Realizada');
    const ticketCards = sellDialog.locator('.events-quick-ticket-card');
    await expect(ticketCards).toHaveCount(2);

    // Cerrar modal de confirmación
    await sellDialog.locator('button:has-text("Listo")').click();
    await expect(sellDialog).not.toBeVisible();

    // 6. Verificar que la tabla de entradas muestra a Ana Cecilia
    const entradasTable = page.locator('.events-table').first();
    await expect(entradasTable).toContainText('Ana Cecilia Valdivia');
    await expect(entradasTable).toContainText('Mesa VIP');
    await expect(entradasTable).toContainText('Pagado');

    // 7. Volver al Resumen y comprobar que la métrica de cobrado es S/ 150.00
    await tabs.filter({ hasText: 'Resumen' }).click();
    const cobradoMetric = page.locator('.events-metric-card').filter({ hasText: 'Cobrado' }).first();
    await expect(cobradoMetric).toContainText('150.00');
  });

  test('Flujo 4: Registro de Gastos y Cálculo de Ganancia Neta', async ({ page }) => {
    await page.goto('/panel#/eventos');
    const eventCard = page.locator('.events-card').filter({ hasText: 'Gala de Shirley Show en Barranco' }).first();
    await eventCard.locator('button:has-text("Ver Detalle")').click();

    const tabs = page.locator('.events-tab-btn');
    await tabs.filter({ hasText: 'Gastos' }).click();

    // Click en Registrar Gasto
    const addExpenseBtn = page.locator('button:has-text("Registrar Gasto")');
    await expect(addExpenseBtn).toBeVisible();
    await addExpenseBtn.click();

    const dialog = page.locator('.events-dialog').first();
    await expect(dialog).toBeVisible();
    await expect(dialog.locator('.events-dialog-title')).toContainText('Registrar Gasto');

    await dialog.locator('input[name="monto"]').fill('50.00');
    await dialog.locator('input[name="descripcion"]').fill('Impresión de afiches y volantes');
    await dialog.locator('select.events-select').selectOption('Publicidad/Marketing');
    await dialog.locator('button[type="submit"]').click();

    await expect(dialog).not.toBeVisible();
    await expect(page.locator('.events-table').first()).toContainText('Impresión de afiches y volantes');
    await expect(page.locator('.events-table').first()).toContainText('50.00');

    // Verificar en Resumen: Cobrado S/ 150 - Gastos S/ 50 = Ganancia Neta S/ 100
    await tabs.filter({ hasText: 'Resumen' }).click();
    const gastoMetric = page.locator('.events-metric-card').filter({ hasText: 'Gastos' }).first();
    await expect(gastoMetric).toContainText('50.00');

    const gananciaMetric = page.locator('.events-metric-card').filter({ hasText: 'Ganancia Neta' });
    await expect(gananciaMetric).toContainText('100.00');
  });

  test('Flujo 5: Interesados (Leads) y Conversión a Venta', async ({ page }) => {
    await page.goto('/panel#/eventos');
    const eventCard = page.locator('.events-card').filter({ hasText: 'Gala de Shirley Show en Barranco' }).first();
    await eventCard.locator('button:has-text("Ver Detalle")').click();

    const tabs = page.locator('.events-tab-btn');
    await tabs.filter({ hasText: 'Interesados' }).click();

    // Registrar interesado
    await page.locator('button:has-text("Registrar Interesado")').click();
    const dialog = page.locator('.events-dialog').first();
    await expect(dialog).toBeVisible();

    await dialog.locator('input[name="nombre"]').fill('Mariana Rios');
    await dialog.locator('input[name="contacto"]').fill('987111222');
    await dialog.locator('input[name="personas"]').fill('1');
    await dialog.locator('button[type="submit"]').click();

    await expect(dialog).not.toBeVisible();
    await expect(page.locator('.events-table').first()).toContainText('Mariana Rios');
    await expect(page.locator('.events-table').first()).toContainText('Pendiente');

    // Convertir a Venta
    const row = page.locator('tr').filter({ hasText: 'Mariana Rios' }).first();
    await row.locator('button:has-text("Convertir")').click();

    const convertDialog = page.locator('.events-dialog').first();
    await expect(convertDialog).toBeVisible();
    await convertDialog.locator('button[type="submit"]:has-text("Confirmar")').click();

    await expect(convertDialog).not.toBeVisible();
    await expect(page.locator('.events-table').first()).toContainText('Confirmado');
  });

  test('Flujo 6: Modo Puerta - Validación de entrada por QR, prevención de reingreso y rechazo de código inválido', async ({ page, request }) => {
    // 1. Obtener el evento para su ID
    const eventsRes = await request.get('/api/eventos');
    const { eventos } = await eventsRes.json();
    const gala = eventos.find(e => e.nombre.includes('Gala de Shirley Show'));
    expect(gala).toBeTruthy();

    // Obtener el secreto real de 32 hex desde SQLite
    const getSecretCmd = `python3 -c "import sqlite3; conn = sqlite3.connect('/tmp/shirley-playwright-data/shirley-show.sqlite3'); cur = conn.cursor(); cur.execute('SELECT codigo_qr FROM entradas WHERE evento_id = ? LIMIT 1', ('${gala.id}',)); print(cur.fetchone()[0])"`;
    const realQrSecret = execSync(getSecretCmd, { encoding: 'utf-8' }).trim();
    expect(realQrSecret).toHaveLength(32);

    // 2. Navegar a Modo Puerta
    await page.goto(`/panel#/puerta/${gala.id}`);
    await expect(page.locator('.door-camera-guide')).toBeVisible();
    await expect(page.locator('.door-scan-hint')).toContainText('Apunta la cámara al código QR');
    // Verificar que no hay campo de texto manual (seguridad endurecida)
    await expect(page.locator('.door-manual-input')).toHaveCount(0);

    // 3. Probar check-in válido vía el endpoint de puerta con el secreto real
    const checkinRes = await request.post('/api/entradas/checkin', {
      data: {
        evento_id: gala.id,
        codigo: realQrSecret,
      }
    });
    const checkinData = await checkinRes.json();
    expect(checkinData.resultado).toBe('Valida');
    expect(['Ana Cecilia Valdivia', 'Mariana Rios']).toContain(checkinData.nombre_comprador);

    // 4. Intentar reingreso con el mismo código: debe ser "Ya Usada"
    const reusedRes = await request.post('/api/entradas/checkin', {
      data: {
        evento_id: gala.id,
        codigo: realQrSecret,
      }
    });
    const reusedData = await reusedRes.json();
    expect(reusedData.resultado).toBe('Ya Usada');

    // 5. Intentar código corto de 8 hex: debe ser "Invalida" por formato
    const shortRes = await request.post('/api/entradas/checkin', {
      data: {
        evento_id: gala.id,
        codigo: 'DEADBEEF',
      }
    });
    const shortData = await shortRes.json();
    expect(shortData.resultado).toBe('Invalida');
    expect(shortData.motivo).toContain('Formato de código inválido');
  });

  test('Flujo 7: Vista Pública de la Entrada Digital para el Asistente', async ({ page, request }) => {
    const eventsRes = await request.get('/api/eventos');
    const { eventos } = await eventsRes.json();
    const gala = eventos.find(e => e.nombre.includes('Gala de Shirley Show'));
    const tixRes = await request.get(`/api/eventos/${gala.id}/entradas`);
    const { entradas } = await tixRes.json();
    const ticket = entradas[0];

    // Navegar a la URL pública del ticket (#/ticket/:token)
    await page.goto(`/panel#/ticket/${ticket.scan_token}`);
    await expect(page.locator('.public-ticket-container')).toBeVisible();
    await expect(page.locator('.public-ticket-event-title')).toContainText('Gala de Shirley Show');
    await expect(page.locator('.public-ticket-info-value').first()).toContainText(ticket.nombre_comprador);
    await expect(page.locator('.public-ticket-card')).toBeVisible();
  });

  test('Flujo 8: Finalizar y Reabrir Evento', async ({ page }) => {
    await page.goto('/panel#/eventos');
    const eventCard = page.locator('.events-card').filter({ hasText: 'Gala de Shirley Show en Barranco' }).first();
    await eventCard.locator('button:has-text("Ver Detalle")').click();

    // Finalizar evento
    const finalizeBtn = page.locator('button:has-text("Finalizar Evento")');
    await expect(finalizeBtn).toBeVisible();
    await finalizeBtn.click();

    // El botón debe cambiar a "Reabrir Evento"
    const reopenBtn = page.locator('button:has-text("Reabrir Evento")');
    await expect(reopenBtn).toBeVisible();

    // En el catálogo debe aparecer en la sección finalizados
    await page.goto('/panel#/eventos');
    await expect(page.locator('.events-card-finalized, .events-badge-muted').first()).toBeVisible();

    // Volver a abrir detalle y reabrir
    await page.locator('.events-card').filter({ hasText: 'Gala de Shirley Show' }).first().locator('button:has-text("Ver Detalle")').click();
    await page.locator('button:has-text("Reabrir Evento")').click();
    await expect(page.locator('button:has-text("Finalizar Evento")')).toBeVisible();
  });

});
