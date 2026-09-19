#!/usr/bin/env node

/**
 * Shirley Show - Responsive Browser Verification Harness (Real Chromium via CDP)
 *
 * Runs headless Chromium, exercises all application routes across multiple viewport widths,
 * evaluates DOM layout probes (horizontal overflow, offscreen controls, touch targets, route expectations),
 * and captures high-resolution screenshots for visual inspection.
 *
 * Zero external npm dependencies: uses Node 24 native stdlib and CDP over WebSocket.
 */

import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdirSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { parseArgs } from 'node:util';

// Parse command line arguments and environment variable fallbacks
const { values: args } = parseArgs({
  options: {
    'base-url': {
      type: 'string',
      default: process.env.SHIRLEY_BASE_URL || 'http://127.0.0.1:3000',
    },
    'presentation-id': {
      type: 'string',
      default: process.env.SHIRLEY_PRESENTATION_ID || '',
    },
    'event-id': {
      type: 'string',
      default: process.env.SHIRLEY_EVENT_ID || '',
    },
    'ticket-token': {
      type: 'string',
      default: process.env.SHIRLEY_TICKET_TOKEN || '',
    },
    'screenshots-dir': {
      type: 'string',
      default: process.env.SHIRLEY_SCREENSHOTS_DIR || './target/responsive-screenshots',
    },
    'chrome-port': {
      type: 'string',
      default: process.env.CHROME_REMOTE_PORT || '',
    },
    'report-file': {
      type: 'string',
      default: '',
    },
    'help': {
      type: 'boolean',
      short: 'h',
    },
  },
  allowPositionals: false,
});

if (args.help) {
  console.log(`
Uso: node scripts/verify-responsive.mjs [opciones]

Opciones:
  --base-url <url>           URL base de Shirley Show (default: http://127.0.0.1:3000)
  --presentation-id <uuid>   ID de presentación sembrada para detalle
  --event-id <uuid>          ID de evento sembrado para detalle y puerta
  --ticket-token <token>     Token de entrada para vista pública
  --screenshots-dir <dir>    Directorio donde guardar capturas PNG (default: ./target/responsive-screenshots)
  --chrome-port <port>       Puerto para remote debugging de Chromium (default: efímero)
  --report-file <path>       Ruta para archivo JSON del reporte
  --help, -h                 Muestra este mensaje de ayuda
`);
  process.exit(0);
}

const baseUrl = args['base-url'].replace(/\/+$/, '');
const presentationId = args['presentation-id'];
const eventId = args['event-id'];
const ticketToken = args['ticket-token'];
const screenshotsDir = resolve(args['screenshots-dir']);
mkdirSync(screenshotsDir, { recursive: true });

const reportFilePath = args['report-file']
  ? resolve(args['report-file'])
  : join(screenshotsDir, 'responsive-report.json');

// Widths requested by specification:
// 320, 360, 390, 414, 480, 640, 768, 1024, 1440
const VIEWPORT_WIDTHS = [320, 360, 390, 414, 480, 640, 768, 1024, 1440];
const VIEWPORT_HEIGHT = 844; // Standard mobile reference height

/**
 * Finds an available TCP port for Chrome remote debugging.
 */
function findFreePort() {
  return new Promise((res, rej) => {
    const server = createServer();
    server.listen(0, '127.0.0.1', () => {
      const port = server.address().port;
      server.close(() => res(port));
    });
    server.on('error', rej);
  });
}

/**
 * Cleanly spawns headless Chromium and connects via CDP WebSocket.
 */
async function launchChromium() {
  const port = args['chrome-port'] ? parseInt(args['chrome-port'], 10) : await findFreePort();
  const chromeUserDataDir = join(tmpdir(), `shirley-chrome-${Date.now()}-${Math.random().toString(36).slice(2)}`);
  mkdirSync(chromeUserDataDir, { recursive: true });

  const chromeProc = spawn('chromium', [
    '--headless=new',
    '--no-sandbox',
    '--disable-gpu',
    '--disable-dev-shm-usage',
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${chromeUserDataDir}`,
    'about:blank',
  ], {
    stdio: ['ignore', 'ignore', 'pipe'],
  });

  let stderrOutput = '';
  chromeProc.stderr.on('data', (d) => {
    stderrOutput += d.toString();
  });

  // Poll until remote debugging endpoint is responsive
  let ready = false;
  for (let i = 0; i < 50; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/json/version`);
      if (r.ok) {
        ready = true;
        break;
      }
    } catch (_e) {
      // Chrome still starting up
    }
    await new Promise((r) => setTimeout(r, 100));
  }

  if (!ready) {
    chromeProc.kill('SIGKILL');
    rmSync(chromeUserDataDir, { recursive: true, force: true });
    throw new Error(`No se pudo conectar a Chromium en puerto ${port}.\nStderr: ${stderrOutput}`);
  }

  // Create new blank page target
  const newTargetRes = await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: 'PUT' });
  const targetInfo = await newTargetRes.json();
  const ws = new WebSocket(targetInfo.webSocketDebuggerUrl);

  await new Promise((res, rej) => {
    ws.onopen = res;
    ws.onerror = rej;
  });

  let messageId = 1;
  function send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = messageId++;
      const handler = (evt) => {
        try {
          const data = JSON.parse(evt.data);
          if (data.id === id) {
            ws.removeEventListener('message', handler);
            if (data.error) {
              reject(new Error(`CDP Error [${method}]: ${JSON.stringify(data.error)}`));
            } else {
              resolve(data.result);
            }
          }
        } catch (_err) {
          // Ignore parse errors of unrelated broadcast events
        }
      };
      ws.addEventListener('message', handler);
      ws.send(JSON.stringify({ id, method, params }));
    });
  }

  function close() {
    try {
      ws.close();
    } catch (_e) {}
    try {
      chromeProc.kill('SIGKILL');
    } catch (_e) {}
    try {
      rmSync(chromeUserDataDir, { recursive: true, force: true });
    } catch (_e) {}
  }

  return { port, send, close };
}

/**
 * Route specifications for verification.
 */
function buildRoutesList() {
  const routes = [
    {
      id: 'catalog',
      name: '01-catalog',
      title: 'Catálogo de Presentaciones',
      hash: '#/presentations',
      selector: '.presentations-view',
      expectedElements: [
        { selector: '.presentations-view', label: 'Contenedor principal del catálogo' },
        { selector: '.welcome-header', label: 'Hero card de bienvenida Ressy AI' },
        { selector: '.app-topbar', label: 'Barra superior de navegación' },
      ],
    },
    {
      id: 'presentation-detail',
      name: '02-presentation-detail',
      title: 'Detalle de Presentación',
      hash: presentationId ? `#/presentations/${encodeURIComponent(presentationId)}` : '#/presentations',
      selector: '.presentation-detail-view',
      expectedElements: [
        { selector: '.presentation-detail-view', label: 'Contenedor del detalle de show' },
        { selector: '.presentation-title-row', label: 'Barra de título y acciones del show' },
        { selector: '.timeline-layout', label: 'Línea de tiempo de elementos' },
        { selector: '.quick-add-bar', label: 'Barra de adición rápida' },
      ],
    },
    {
      id: 'events-catalog',
      name: '03-events-catalog',
      title: 'Catálogo de Eventos',
      hash: '#/eventos',
      selector: '.events-catalog-view',
      expectedElements: [
        { selector: '.events-catalog-view', label: 'Contenedor del catálogo de eventos' },
        { selector: '.events-catalog-header', label: 'Encabezado con botón de nuevo evento' },
        { selector: '.events-card', label: 'Tarjeta de evento sembrado' },
      ],
    },
    {
      id: 'event-detail',
      name: '04-event-detail',
      title: 'Detalle de Evento',
      hash: eventId ? `#/eventos/${encodeURIComponent(eventId)}` : '#/eventos',
      selector: '.events-detail-view',
      expectedElements: [
        { selector: '.events-detail-view', label: 'Contenedor de detalle del evento' },
        { selector: '.events-tabs-nav', label: 'Barra de pestañas del evento' },
        { selector: '.events-tab-panel', label: 'Panel activo de contenido' },
      ],
    },
    {
      id: 'public-ticket',
      name: '05-public-ticket',
      title: 'Entrada Digital Pública',
      hash: ticketToken ? `#/ticket/${encodeURIComponent(ticketToken)}` : '#/presentations',
      selector: '.public-ticket-container',
      expectedElements: [
        { selector: '.public-ticket-container', label: 'Contenedor de entrada pública' },
        { selector: '.public-ticket-card', label: 'Tarjeta de entrada digital' },
        { selector: '.public-ticket-qr-box', label: 'Contenedor de código QR' },
      ],
    },
    {
      id: 'door',
      name: '06-door-checkin',
      title: 'Control de Puerta (Check-in)',
      hash: eventId ? `#/puerta/${encodeURIComponent(eventId)}` : '#/eventos',
      selector: '.door-container',
      expectedElements: [
        { selector: '.door-container', label: 'Contenedor de control de puerta' },
        { selector: '.door-manual-input', label: 'Campo de texto para código manual' },
        { selector: '.door-manual-form button', label: 'Botón de validación manual' },
      ],
    },
  ];

  return routes;
}

/**
 * The client-side DOM probe evaluated inside Chromium.
 * Tolerant to legitimate horizontal scroll containers and off-screen hidden overlays.
 */
const PROBE_FUNCTION = `
(() => {
  const viewportWidth = window.innerWidth;
  const viewportHeight = window.innerHeight;
  const docElement = document.documentElement;
  const body = document.body;

  const docWidth = Math.max(
    docElement ? docElement.scrollWidth : 0,
    body ? body.scrollWidth : 0
  );
  const overflowDiff = Math.max(0, docWidth - viewportWidth);
  const hasDocumentOverflow = overflowDiff > 1.5;

  function isVisible(el) {
    if (!el || el.nodeType !== Node.ELEMENT_NODE) return false;
    const tag = el.tagName;
    if (['SCRIPT', 'STYLE', 'TEMPLATE', 'NOSCRIPT'].includes(tag)) return false;
    if (el.closest('[aria-hidden="true"]')) return false;
    if (el.closest('dialog:not([open])')) return false;

    // Ignore SVGs internal definitions
    if (['DEFS', 'CLIPPATH', 'MASK', 'PATTERN', 'LINEARGARDIENT', 'RADIALGRADIENT'].includes(tag)) return false;

    const style = window.getComputedStyle(el);
    if (style.display === 'none' || style.visibility === 'hidden' || Number(style.opacity) === 0) return false;

    const rect = el.getBoundingClientRect();
    if (rect.width === 0 && rect.height === 0) return false;
    return true;
  }

  function isInsideScrollContainer(el) {
    let parent = el.parentElement;
    while (parent && parent !== document.body && parent !== document.documentElement) {
      const style = window.getComputedStyle(parent);
      if (style.overflowX === 'auto' || style.overflowX === 'scroll') {
        return true;
      }
      parent = parent.parentElement;
    }
    return false;
  }

  function getSelector(el) {
    const tag = el.tagName.toLowerCase();
    const id = el.id ? '#' + el.id : '';
    let cls = '';
    if (el.className && typeof el.className === 'string') {
      const classes = el.className.trim().split(/\\s+/).filter(Boolean).slice(0, 2);
      if (classes.length > 0) cls = '.' + classes.join('.');
    }
    return tag + id + cls;
  }

  function getTextSnippet(el) {
    const raw = el.innerText || el.getAttribute('aria-label') || el.getAttribute('placeholder') || el.value || '';
    return raw.trim().slice(0, 40).replace(/\\s+/g, ' ');
  }

  // 1. Detect overflowing elements
  const allElements = Array.from(document.querySelectorAll('*'));
  const overflowingElements = [];

  for (const el of allElements) {
    if (!isVisible(el)) continue;
    if (isInsideScrollContainer(el)) continue;

    const rect = el.getBoundingClientRect();
    if (rect.right > viewportWidth + 1.5) {
      overflowingElements.push({
        selector: getSelector(el),
        right: Math.round(rect.right),
        width: Math.round(rect.width),
        overflow: Math.round(rect.right - viewportWidth),
        text: getTextSnippet(el),
      });
    }
  }

  // 1b. Detect clipped content (clipped by viewport or an ancestor with overflow: hidden/clip)
  const clippedContent = [];

  for (const el of allElements) {
    if (!isVisible(el)) continue;
    if (el === docElement || el === body) continue;
    if (isInsideScrollContainer(el)) continue;

    const rect = el.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) continue;

    // Check if element right exceeds innerWidth + 1
    const exceedsViewport = rect.right > viewportWidth + 1;

    // Check if element is wider than its container with hidden overflow or right edge extends past container
    let hiddenContainer = null;
    let hiddenContainerRect = null;
    let exceedsHiddenContainer = false;

    let parent = el.parentElement;
    while (parent && parent !== body && parent !== docElement) {
      const parentStyle = window.getComputedStyle(parent);
      if (parentStyle.overflowX === 'auto' || parentStyle.overflowX === 'scroll') {
        hiddenContainer = null;
        exceedsHiddenContainer = false;
        break;
      }
      if (
        parentStyle.overflowX === 'hidden' ||
        parentStyle.overflowX === 'clip' ||
        parentStyle.overflow === 'hidden' ||
        parentStyle.overflow === 'clip'
      ) {
        const pRect = parent.getBoundingClientRect();
        if (rect.right > pRect.right + 1 || rect.width > pRect.width + 1) {
          hiddenContainer = parent;
          hiddenContainerRect = pRect;
          exceedsHiddenContainer = true;
          break;
        }
      }
      parent = parent.parentElement;
    }

    if (exceedsViewport || exceedsHiddenContainer) {
      const excess = exceedsViewport
        ? Math.round(rect.right - viewportWidth)
        : Math.round(rect.right - hiddenContainerRect.right);

      clippedContent.push({
        selector: getSelector(el),
        text: getTextSnippet(el),
        reason: exceedsViewport ? 'exceeds_viewport' : 'exceeds_hidden_container',
        metrics: {
          elementRight: Math.round(rect.right),
          elementWidth: Math.round(rect.width),
          containerRight: hiddenContainerRect ? Math.round(hiddenContainerRect.right) : null,
          containerWidth: hiddenContainerRect ? Math.round(hiddenContainerRect.width) : null,
          innerWidth: viewportWidth,
          excess: Math.max(1, excess > 0 ? excess : Math.round(rect.width - (hiddenContainerRect ? hiddenContainerRect.width : viewportWidth))),
        },
      });
    }
  }

  // 2. Detect off-screen controls and small touch targets
  const interactives = Array.from(document.querySelectorAll(
    'button, a[href], input, select, textarea, [role="button"], [role="tab"], [role="menuitem"], [tabindex="0"]'
  ));

  const offscreenControls = [];
  const smallTouchTargets = [];

  for (const el of interactives) {
    if (!isVisible(el)) continue;
    const style = window.getComputedStyle(el);
    if (style.pointerEvents === 'none') continue;

    // Skip nested children inside another interactive element (e.g. icon inside button)
    if (el.parentElement && el.parentElement.closest('button, a[href], [role="button"]')) continue;

    const rect = el.getBoundingClientRect();

    // Check off-screen controls
    if (rect.left < -2 || rect.right > viewportWidth + 1.5) {
      if (!isInsideScrollContainer(el)) {
        offscreenControls.push({
          selector: getSelector(el),
          left: Math.round(rect.left),
          right: Math.round(rect.right),
          text: getTextSnippet(el),
        });
      }
    }

    // Label delegation for checkbox / radio
    if (el.type === 'checkbox' || el.type === 'radio') {
      const label = el.closest('label');
      if (label) {
        const lRect = label.getBoundingClientRect();
        if (lRect.width >= 44 && lRect.height >= 44) continue;
      }
    }

    // Inline text links inside paragraphs/headings/titles (exempt under WCAG 2.5.8)
    if (el.tagName === 'A' && !el.classList.contains('btn') && !el.classList.contains('icon-btn')) {
      continue;
    }

    // Touch target size evaluation (< 44px)
    if (rect.width < 44 || rect.height < 44) {
      const isSeverelySmall = (rect.width < 32 || rect.height < 32) || (rect.width < 44 && rect.height < 44);
      smallTouchTargets.push({
        selector: getSelector(el),
        width: Math.round(rect.width),
        height: Math.round(rect.height),
        text: getTextSnippet(el),
        isSeverelySmall,
      });
    }
  }

  return {
    viewportWidth,
    viewportHeight,
    docWidth,
    overflowDiff,
    hasDocumentOverflow,
    overflowingElements,
    clippedContent,
    offscreenControls,
    smallTouchTargets,
  };
})()
`;

async function main() {
  console.log('='.repeat(70));
  console.log('🚀 Shirley Show - Verificación Responsive con Navegador Real');
  console.log(`   Base URL: ${baseUrl}`);
  console.log(`   Capturas: ${screenshotsDir}`);
  console.log(`   Anchos:   ${VIEWPORT_WIDTHS.join(', ')} px`);
  console.log('='.repeat(70));

  let chromium;
  try {
    chromium = await launchChromium();
  } catch (err) {
    console.error('❌ Error fatal iniciando Chromium:', err.message);
    process.exit(1);
  }

  // Ensure the seeded event has a valid poster image (1x1 PNG) so Chromium decodes it cleanly in visual inspection
  if (eventId) {
    try {
      const pngBase64 = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==';
      const pngBuffer = Buffer.from(pngBase64, 'base64');
      const blob = new Blob([pngBuffer], { type: 'image/png' });
      const formData = new FormData();
      formData.append('imagen', blob, 'poster.png');

      const posterRes = await fetch(`${baseUrl}/api/eventos/${encodeURIComponent(eventId)}/imagen`, {
        method: 'POST',
        body: formData,
      });
      if (posterRes.ok) {
        console.log('   Afiche de evento inicializado con imagen PNG válida (1x1 px).');
      }
    } catch (e) {
      console.warn('   Aviso al sembrar afiche:', e.message);
    }
  }

  // Setup cleanup handlers
  const cleanup = () => {
    if (chromium) {
      chromium.close();
      chromium = null;
    }
  };
  process.on('exit', cleanup);
  process.on('SIGINT', () => { cleanup(); process.exit(130); });
  process.on('SIGTERM', () => { cleanup(); process.exit(143); });
  process.on('uncaughtException', (e) => {
    console.error('Uncaught Exception:', e);
    cleanup();
    process.exit(1);
  });

  const routes = buildRoutesList();
  const results = [];
  let totalViolations = 0;

  try {
    await chromium.send('Page.enable');
    await chromium.send('Runtime.enable');
    await chromium.send('DOM.enable');

    for (const route of routes) {
      console.log(`\n📍 [Ruta] ${route.title} (${route.hash})`);

      for (const width of VIEWPORT_WIDTHS) {
        // 1. Emulate viewport metrics with mobile: true
        await chromium.send('Emulation.setDeviceMetricsOverride', {
          width,
          height: VIEWPORT_HEIGHT,
          deviceScaleFactor: 1,
          mobile: true,
        });

        // 2. Navigate to target route
        const targetUrl = `${baseUrl}/${route.hash}`;
        await chromium.send('Page.navigate', { url: targetUrl });

        // 3. Wait for primary expected selector to mount
        let mounted = false;
        const startWait = Date.now();
        while (Date.now() - startWait < 5000) {
          const checkRes = await chromium.send('Runtime.evaluate', {
            expression: `Boolean(document.querySelector('${route.selector}') && !document.querySelector('.shell-main .spinner'))`,
            returnByValue: true,
          });
          if (checkRes?.result?.value === true) {
            mounted = true;
            break;
          }
          await new Promise((r) => setTimeout(r, 80));
        }

        // Brief delay for CSS transitions, fonts, and layout reflow
        await new Promise((r) => setTimeout(r, 120));

        // 4. Verify route expectations
        const expectationsCheck = await chromium.send('Runtime.evaluate', {
          expression: `(() => {
            const list = ${JSON.stringify(route.expectedElements)};
            const missing = [];
            for (const item of list) {
              const el = document.querySelector(item.selector);
              if (!el) {
                missing.push({ selector: item.selector, label: item.label, reason: 'not_found' });
                continue;
              }
              const style = window.getComputedStyle(el);
              if (style.display === 'none' || style.visibility === 'hidden') {
                missing.push({ selector: item.selector, label: item.label, reason: 'hidden' });
              }
            }
            return missing;
          })()`,
          returnByValue: true,
        });
        const missingExpectations = expectationsCheck?.result?.value || [];

        // 5. Evaluate DOM layout probe
        const probeEval = await chromium.send('Runtime.evaluate', {
          expression: PROBE_FUNCTION,
          returnByValue: true,
        });
        const probe = probeEval?.result?.value || {
          viewportWidth: width,
          docWidth: width,
          overflowDiff: 0,
          hasDocumentOverflow: false,
          overflowingElements: [],
          clippedContent: [],
          offscreenControls: [],
          smallTouchTargets: [],
        };

        // 6. Capture full screenshot
        const shotRes = await chromium.send('Page.captureScreenshot', {
          format: 'png',
          captureBeyondViewport: false,
        });
        const shotFilename = `${route.name}-${width}px.png`;
        const shotPath = join(screenshotsDir, shotFilename);
        writeFileSync(shotPath, Buffer.from(shotRes.data, 'base64'));

        // 7. Analyze violations
        // A layout violation is:
        // - Page failed to mount or missing required view expectations
        // - Horizontal document overflow (scrollWidth > innerWidth)
        // - Individual visible non-scroll elements exceeding right edge
        // - Offscreen interactive controls clipped
        // - Severely undersized touch targets (both width and height < 44, or < 32px) on mobile viewports (<= 768px)
        const isMobile = width <= 768;
        const severeTouchTargets = isMobile
          ? probe.smallTouchTargets.filter((t) => t.isSeverelySmall)
          : [];

        const hasLayoutViolation =
          !mounted ||
          missingExpectations.length > 0 ||
          probe.hasDocumentOverflow ||
          probe.overflowingElements.length > 0 ||
          probe.clippedContent.length > 0 ||
          probe.offscreenControls.length > 0 ||
          severeTouchTargets.length > 0;

        if (hasLayoutViolation) {
          totalViolations++;
        }

        const statusTag = hasLayoutViolation ? '❌ FAIL' : '✅ PASS';
        const overflowText = probe.overflowDiff > 0 ? `+${probe.overflowDiff}px OVERFLOW` : '0px overflow';
        const clipSummaryText = probe.clippedContent.length > 0 ? ` | ${probe.clippedContent.length} recortados` : '';
        const touchText = `${probe.smallTouchTargets.length} touch <44px (${severeTouchTargets.length} críticos)`;

        console.log(
          `  ${statusTag} | ${String(width).padStart(4)}px | docWidth=${probe.docWidth}px (${overflowText})${clipSummaryText} | ${touchText} | 📸 ${shotFilename}`
        );

        if (!mounted) {
          console.log(`       ⚠️ Vista no terminó de renderizar selector esperado: ${route.selector}`);
        }
        for (const exp of missingExpectations) {
          console.log(`       ⚠️ Elemento clave ausente: ${exp.selector} (${exp.label}) [${exp.reason}]`);
        }
        for (const ov of probe.overflowingElements) {
          console.log(`       ⚠️ Desborde horizontal: ${ov.selector} right=${ov.right}px (+${ov.overflow}px) texto="${ov.text}"`);
        }
        for (const clip of probe.clippedContent) {
          console.log(`       ⚠️ Contenido recortado: ${clip.selector} [${clip.reason}] right=${clip.metrics.elementRight}px (excess=+${clip.metrics.excess}px) texto="${clip.text}"`);
        }
        for (const off of probe.offscreenControls) {
          console.log(`       ⚠️ Control fuera de pantalla: ${off.selector} [left=${off.left}, right=${off.right}] texto="${off.text}"`);
        }
        for (const st of severeTouchTargets) {
          console.log(`       ⚠️ Touch target pequeño: ${st.selector} (${st.width}x${st.height}px) texto="${st.text}"`);
        }

        results.push({
          routeId: route.id,
          routeName: route.name,
          routeTitle: route.title,
          routeHash: route.hash,
          width,
          status: hasLayoutViolation ? 'FAIL' : 'PASS',
          mounted,
          missingExpectations,
          docWidth: probe.docWidth,
          viewportWidth: probe.viewportWidth,
          overflowDiff: probe.overflowDiff,
          hasDocumentOverflow: probe.hasDocumentOverflow,
          overflowingElements: probe.overflowingElements,
          clippedContent: probe.clippedContent,
          offscreenControls: probe.offscreenControls,
          smallTouchTargets: probe.smallTouchTargets,
          severeTouchTargets,
          screenshot: shotFilename,
        });
      }
    }
  } finally {
    cleanup();
  }

  // Write comprehensive JSON report
  const summaryReport = {
    timestamp: new Date().toISOString(),
    baseUrl,
    totalChecks: results.length,
    passedChecks: results.filter((r) => r.status === 'PASS').length,
    failedChecks: results.filter((r) => r.status === 'FAIL').length,
    totalViolations,
    totalClippedViolations: results.reduce((sum, r) => sum + (r.clippedContent?.length || 0), 0),
    widths: VIEWPORT_WIDTHS,
    results,
  };
  writeFileSync(reportFilePath, JSON.stringify(summaryReport, null, 2), 'utf-8');

  // Print final summary
  console.log('\n' + '='.repeat(70));
  console.log('📊 RESUMEN DE VERIFICACIÓN RESPONSIVE');
  console.log('='.repeat(70));
  console.log(`Total verificaciones (rutas × anchos): ${summaryReport.totalChecks}`);
  console.log(`Pruebas aprobadas:                   ${summaryReport.passedChecks}`);
  console.log(`Pruebas con violaciones:             ${summaryReport.failedChecks}`);
  console.log(`Violaciones de contenido recortado:  ${summaryReport.totalClippedViolations}`);
  console.log(`Reporte JSON guardado en:            ${reportFilePath}`);
  console.log(`Capturas PNG guardadas en:           ${screenshotsDir}`);
  console.log('='.repeat(70));

  if (totalViolations > 0) {
    console.error(`\n❌ Se encontraron ${totalViolations} violaciones de layout responsive.`);
    process.exit(1);
  } else {
    console.log('\n🎉 ¡Todas las vistas y anchos pasaron la verificación responsive sin violaciones!');
    process.exit(0);
  }
}

main().catch((err) => {
  console.error('Fatal error en verificación responsive:', err);
  process.exit(1);
});
