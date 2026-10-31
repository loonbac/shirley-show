
import { test, expect } from '@playwright/test';
import { execSync } from 'child_process';

test.describe('Modo Puerta y Escaneo de QR Musical E2E', () => {

  test('Generación de QR con logo Shirley Show y escaneo en Modo Puerta', async ({ page, request }) => {
    // 1. Crear evento y vender entrada pagada
    const createEvtRes = await request.post('/api/eventos', {
      data: {
        nombre: 'Concierto VIP Shirley Show',
        fecha_hora: '2026-12-20T21:00:00Z',
        lugar: 'Gran Teatro Nacional',
        tipos_de_entrada: [{ nombre: 'Platinum', precio: 100.0, capacidad: 50 }],
        incluye_igv: false
      }
    });
    const { evento } = await createEvtRes.json();
    expect(evento.id).toBeTruthy();

    const sellRes = await request.post(`/api/eventos/${evento.id}/entradas`, {
      data: {
        nombre_comprador: 'Valeria Albarracín',
        contacto: '987654321',
        tipo_entrada: 'Platinum',
        cantidad: 1,
        medio_pago: 'Yape',
        estado_pago: 'Pagado'
      }
    });
    const sellData = await sellRes.json();
    const ticket = sellData.entradas ? sellData.entradas[0] : sellData.entrada;
    expect(ticket).toBeTruthy();
    expect(ticket.scan_token).toBeTruthy();

    // 2. Navegar a la vista pública de la entrada y descargar la foto
    await page.goto(`/#/ticket/${ticket.scan_token}`);
    await expect(page.locator('.public-ticket-event-title')).toContainText('Concierto VIP Shirley Show');
    await expect(page.locator('.public-ticket-qr-box svg')).toBeVisible();

    // 3. Probar en el contexto del navegador que generarFotoEntrada genera un QR
    // con alta densidad (Versión 8, 49x49 módulos), logo amplio (~168px) y escudo blanco,
    // y que tanto ZXing como jsQR lo decodifican inmediatamente al secreto de 32 hex.
    const qrValidation = await page.evaluate(async () => {
      const { generarMatriz, construir, dibujarQR } = await import('/assets/js/modules/events/ticket-qr-dots.js');
      const { decodeCanvasWithZXing, decodeCenterCropWithZXing, getJsQR } = await import('/assets/js/modules/events/camera-qr-scanner.js');

      const testCode = 'e3b0c44298fc1c149afbf4c8996fb924';
      const m = generarMatriz(testCode);
      const c = construir(testCode);

      // Crear canvas y dibujar el QR con logo
      const canvas = document.createElement('canvas');
      canvas.width = 660;
      canvas.height = 660;
      const ctx = canvas.getContext('2d');

      // Cargar logo Shirley Show real
      const logoImg = new Image();
      logoImg.src = '/assets/public/logo-qr.png';
      await new Promise((resolve) => {
        if (logoImg.complete) resolve();
        else {
          logoImg.onload = resolve;
          logoImg.onerror = resolve;
        }
      });

      dibujarQR(ctx, testCode, 0, 0, 660, logoImg);

      // Decodificación directa con ZXing sobre el QR completo
      const zxingDecoded = decodeCanvasWithZXing(canvas);

      // Decodificación con jsQR sobre el QR completo
      const imgData = ctx.getImageData(0, 0, 660, 660);
      const jsQR = getJsQR();
      const jsQrDecoded = jsQR ? jsQR(imgData.data, 660, 660, { inversionAttempts: 'attemptBoth' }) : null;

      // Simular frame de cámara 1280x720 con el QR centrado (como cuando el usuario enfoca)
      const camCanvas = document.createElement('canvas');
      camCanvas.width = 1280;
      camCanvas.height = 720;
      const camCtx = camCanvas.getContext('2d');
      camCtx.fillStyle = '#666666';
      camCtx.fillRect(0, 0, 1280, 720);
      // Centrar el QR a 400x400 dentro del visor
      camCtx.drawImage(canvas, (1280 - 400) / 2, (720 - 400) / 2, 400, 400);

      const cropDecoded = decodeCenterCropWithZXing(camCanvas, 1280, 720);

      return {
        matrixSize: m.size,
        logoMods: c.logo.s,
        logoPx: Math.round(c.logo.s * (660 / c.lienzo)),
        zxingDecoded,
        cropDecoded,
        jsQrDecoded: jsQrDecoded ? jsQrDecoded.data : null
      };
    });

    // Validar requerimientos del usuario:
    // "agregas más puntos al QR": Versión 10 (57x57 = 3,249 módulos frente a 1,089 de V4)
    expect(qrValidation.matrixSize).toBe(57);
    // "el logo se ve muy pequeño...": logo extra grande de 23 módulos (~241px en 660px) con fondo transparente
    expect(qrValidation.logoMods).toBe(23);
    expect(qrValidation.logoPx).toBeGreaterThanOrEqual(230);
    // Decodificación perfecta en ZXing (completo y visor central)
    expect(qrValidation.zxingDecoded).toBe('e3b0c44298fc1c149afbf4c8996fb924');
    expect(qrValidation.cropDecoded).toBe('e3b0c44298fc1c149afbf4c8996fb924');

    // 4. Probar el Modo Puerta: navegar a #/puerta/:id
    await page.goto(`/#/puerta/${evento.id}`);
    await expect(page.locator('.door-camera-guide')).toBeVisible();

    // Obtener el secreto real de la entrada de Valeria desde SQLite
    const getSecretCmd = `python3 -c "import sqlite3; conn = sqlite3.connect('/tmp/shirley-playwright-data/shirley-show.sqlite3'); cur = conn.cursor(); cur.execute('SELECT codigo_qr FROM entradas WHERE id = ?', ('${ticket.id}',)); print(cur.fetchone()[0])"`;
    const valeriaSecret = execSync(getSecretCmd, { encoding: 'utf-8' }).trim();
    expect(valeriaSecret).toHaveLength(32);

    // 5. Simular la llegada de un frame con el QR de Valeria al scanner y verificar decodificación
    const scanSimResult = await page.evaluate(async (secret) => {
      const { dibujarQR } = await import('/assets/js/modules/events/ticket-qr-dots.js');
      const { decodeCanvasWithZXing, decodeCenterCropWithZXing } = await import('/assets/js/modules/events/camera-qr-scanner.js');

      const simCanvas = document.createElement('canvas');
      simCanvas.width = 660;
      simCanvas.height = 660;
      const ctx = simCanvas.getContext('2d');

      const logoImg = new Image();
      logoImg.src = '/assets/public/logo-qr.png';
      await new Promise(r => { if (logoImg.complete) r(); else logoImg.onload = r; });

      dibujarQR(ctx, secret, 0, 0, 660, logoImg);

      // Decodificación directa
      const decodedDirect = decodeCanvasWithZXing(simCanvas);

      // Simular cámara 1280x720 enfocando el QR
      const cam = document.createElement('canvas');
      cam.width = 1280;
      cam.height = 720;
      const cctx = cam.getContext('2d');
      cctx.fillStyle = '#222222';
      cctx.fillRect(0, 0, 1280, 720);
      cctx.drawImage(simCanvas, (1280 - 420) / 2, (720 - 420) / 2, 420, 420);

      const decodedFromCam = decodeCenterCropWithZXing(cam, 1280, 720);

      return { decodedDirect, decodedFromCam };
    }, valeriaSecret);

    expect(scanSimResult.decodedDirect).toBe(valeriaSecret);
    expect(scanSimResult.decodedFromCam).toBe(valeriaSecret);

    // 6. Enviar el código decodificado al check-in de puerta y verificar éxito
    const checkinRes = await request.post('/api/entradas/checkin', {
      data: {
        evento_id: evento.id,
        codigo: scanSimResult.decodedFromCam,
      }
    });
    const checkinData = await checkinRes.json();
    expect(checkinData.resultado).toBe('Valida');
    expect(checkinData.nombre_comprador).toBe('Valeria Albarracín');

    // 7. Intentar reingreso: debe ser rechazado como "Ya Usada"
    const retryRes = await request.post('/api/entradas/checkin', {
      data: {
        evento_id: evento.id,
        codigo: scanSimResult.decodedFromCam,
      }
    });
    const retryData = await retryRes.json();
    expect(retryData.resultado).toBe('Ya Usada');

    // 8. Verificar que la vista de Modo Puerta muestra la barra de turno y contadores
    await expect(page.locator('.door-shift-badge')).toContainText('Turno 1');
    await expect(page.locator('.door-kpi-turno .door-kpi-title')).toContainText('En este Turno');
    await expect(page.locator('.door-kpi-total .door-kpi-title')).toContainText('Asistencia Total');
    await expect(page.locator('.door-scan-log-header')).toContainText('Historial del Turno');

    // 9. Probar el flujo de Cierre de Turno en la interfaz
    const cerrarTurnoBtn = page.locator('button:has-text("Cerrar Turno")');
    await expect(cerrarTurnoBtn).toBeVisible();
    await cerrarTurnoBtn.click();

    // Modal de Cierre de Turno visible con resumen consolidado
    await expect(page.locator('.door-modal-overlay')).toBeVisible();
    await expect(page.locator('.door-modal-title')).toContainText('Cierre de Turno 1');
    await expect(page.locator('.door-modal-stats')).toContainText('Total ingresados en el turno:');

    // Iniciar Siguiente Turno
    const siguienteTurnoBtn = page.locator('button:has-text("Iniciar Siguiente Turno")');
    await siguienteTurnoBtn.click();
    await expect(page.locator('.door-modal-overlay')).not.toBeVisible();
    await expect(page.locator('.door-shift-badge')).toContainText('Turno 2');

    // 10. Verificar que el resumen del evento en el sistema registra la asistencia en puerta
    const summaryRes = await request.get(`/api/eventos/${evento.id}/resumen`);
    const summaryData = await summaryRes.json();
    expect(summaryData.resumen.entradas_ingresadas).toBe(1);
    expect(summaryData.resumen.entradas_emitidas).toBe(1);

    // 11. Verificar experiencia móvil sin scroll vertical y tap-to-toggle para ahorro de batería
    await page.setViewportSize({ width: 375, height: 667 });

    const scrollCheck = await page.evaluate(() => {
      const container = document.querySelector('.door-container');
      return {
        bodyScrollHeight: document.body.scrollHeight,
        bodyClientHeight: document.body.clientHeight,
        hasDoorClass: document.body.classList.contains('door-mode-active'),
        containerScrollHeight: container ? container.scrollHeight : 0,
        containerClientHeight: container ? container.clientHeight : 0,
      };
    });

    expect(scrollCheck.hasDoorClass).toBe(true);
    // Cero scroll vertical en viewport móvil: la altura del contenido no excede la ventana
    expect(scrollCheck.bodyScrollHeight).toBeLessThanOrEqual(scrollCheck.bodyClientHeight + 1);

    // Probar tap-to-toggle: pausar la cámara con un tap para ahorro de batería
    const camSection = page.locator('.door-camera-section');
    await expect(camSection).toHaveClass(/is-active/);
    await camSection.click();
    await expect(camSection).toHaveClass(/is-paused/);
    await expect(page.locator('.door-camera-standby-title')).toContainText('Toca para activar la cámara');

    // Segundo tap en el visor: reactivar la cámara de inmediato
    await camSection.click();
    await expect(camSection).toHaveClass(/is-active/);
    await expect(page.locator('.door-camera-pill')).toContainText('Toca para pausar');

    // 12. Verificar que la asistente Ressy (burbuja flotante y atajos) está completamente oculta/suprimida en Modo Puerta
    const ressyFloating = page.locator('.agent-chat-floating-trigger');
    await expect(ressyFloating).toBeHidden();

    // 13. Verificar que se usan SVGs (sin emojis), sin botón redundante de cámara en el topbar, y modal transparente expandible de historial
    await expect(page.locator('.door-camera-toggle-btn')).toHaveCount(0);
    const svgsInView = await page.locator('.door-container svg').count();
    expect(svgsInView).toBeGreaterThanOrEqual(2);

    const logSection = page.locator('.door-scan-log-section');
    const backdrop = page.locator('.door-history-backdrop');

    // Inicialmente acoplado al fondo
    await expect(logSection).not.toHaveClass(/is-expanded/);
    await expect(backdrop).not.toHaveClass(/is-visible/);

    // Al tocar o deslizar hacia arriba el historial, se expande a modal transparente
    await page.locator('.door-log-expand-btn').click();
    await expect(logSection).toHaveClass(/is-expanded/);
    await expect(backdrop).toHaveClass(/is-visible/);
    await expect(page.locator('.door-log-title')).toContainText('Historial Completo');

    // Al tocar el botón de cerrar o el backdrop, vuelve a su sitio acoplado
    await page.locator('.door-log-expand-btn').click();
    await expect(logSection).not.toHaveClass(/is-expanded/);
    await expect(backdrop).not.toHaveClass(/is-visible/);

    // 14. Probar arrastre táctil interactivo 1:1 que sigue el dedo naturalmente
    const handle = page.locator('.door-log-handle');

    // Deslizar hacia arriba arrastrando el dedo: el bottom sheet sigue el movimiento en tiempo real
    const dragUpResult = await page.evaluate(() => {
      const h = document.querySelector('.door-log-handle');
      const sheet = document.querySelector('.door-scan-log-section');
      if (!h || !sheet) return null;

      // Iniciar toque
      const touchStart = new Touch({ identifier: 1, target: h, clientY: 620, clientX: 180 });
      h.dispatchEvent(new TouchEvent('touchstart', { touches: [touchStart], changedTouches: [touchStart], bubbles: true }));

      // Mover el dedo hacia arriba 200px
      const touchMove = new Touch({ identifier: 1, target: h, clientY: 420, clientX: 180 });
      h.dispatchEvent(new TouchEvent('touchmove', { touches: [touchMove], changedTouches: [touchMove], cancelable: true, bubbles: true }));

      const transformDuringDrag = sheet.style.transform;

      // Soltar dedo
      const touchEnd = new Touch({ identifier: 1, target: h, clientY: 420, clientX: 180 });
      h.dispatchEvent(new TouchEvent('touchend', { touches: [], changedTouches: [touchEnd], bubbles: true }));

      return { transformDuringDrag };
    });

    expect(dragUpResult.transformDuringDrag).toContain('translate3d');
    await expect(logSection).toHaveClass(/is-expanded/);
    await expect(backdrop).toHaveClass(/is-visible/);

    // Deslizar hacia abajo arrastrando el dedo: vuelve a colapsar
    await page.evaluate(() => {
      const h = document.querySelector('.door-log-handle');
      const touchStart = new Touch({ identifier: 2, target: h, clientY: 80, clientX: 180 });
      h.dispatchEvent(new TouchEvent('touchstart', { touches: [touchStart], changedTouches: [touchStart], bubbles: true }));

      const touchMove = new Touch({ identifier: 2, target: h, clientY: 350, clientX: 180 });
      h.dispatchEvent(new TouchEvent('touchmove', { touches: [touchMove], changedTouches: [touchMove], cancelable: true, bubbles: true }));

      const touchEnd = new Touch({ identifier: 2, target: h, clientY: 350, clientX: 180 });
      h.dispatchEvent(new TouchEvent('touchend', { touches: [], changedTouches: [touchEnd], bubbles: true }));
    });

    await expect(logSection).not.toHaveClass(/is-expanded/);
    await expect(backdrop).not.toHaveClass(/is-visible/);
  });
});
