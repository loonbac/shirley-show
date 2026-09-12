/**
 * Pure generator and sharing helper for WhatsApp ticket photo cards.
 * Warm ivory and antique gold palette (#b8860b) for Shirley Show.
 * Renders attendees, event data, ticket type, short code, and vector QR onto canvas (1080px width).
 */

const ANCHO = 1080;
const MARGEN = 80;
const QR_PX = 620;
const MARCO_PADDING = 36;
const MARCO_SIZE = QR_PX + (MARCO_PADDING * 2); // 692px

// Warm Shirley Show palette tokens (matching tokens/colors.css)
const COLOR_BG = "#faf8f5";              // Warm Ivory
const COLOR_SURFACE = "#ffffff";         // Pure White for QR backdrop
const COLOR_BORDE = "#e2dbce";           // Warm subtle border
const COLOR_TEXTO = "#1c1917";           // Warm Charcoal
const COLOR_SUAVE = "#574f45";           // Muted Charcoal
const COLOR_DIM = "#7d7367";             // Dim Charcoal
const COLOR_ORO = "#b8860b";             // Sober Metallic Antique Gold
const COLOR_ORO_HOVER = "#9a7209";       // Deep Gold
const COLOR_PASTILLA_BG = "#fcf8ed";     // Warm Gold Subtle Tint
const COLOR_PASTILLA_BORDE = "#b8860b";  // Gold Border

const FUENTE = "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif";

/**
 * Normalizes SVG text by guaranteeing valid XML namespace and explicit dimensions.
 * @param {string} svgString 
 * @returns {string}
 */
export function normalizarSvg(svgString) {
  let svg = String(svgString || "").trim();
  if (!svg.includes("xmlns=")) {
    svg = svg.replace(/<svg\b/i, '<svg xmlns="http://www.w3.org/2000/svg"');
  }
  if (!svg.match(/<svg[^>]*\bwidth\s*=/i)) {
    svg = svg.replace(/<svg\b/i, '<svg width="600" height="600"');
  }
  return svg;
}

/**
 * Asynchronously rasterizes an SVG string into an HTMLImageElement using an object URL.
 * @param {string} svgString 
 * @returns {Promise<HTMLImageElement>}
 */
export function cargarImagenDesdeSvg(svgString) {
  return new Promise((resolve, reject) => {
    if (!svgString) {
      return reject(new Error("SVG no disponible"));
    }
    try {
      const cleanSvg = normalizarSvg(svgString);
      const blob = new Blob([cleanSvg], { type: "image/svg+xml;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const img = new Image();

      img.onload = () => {
        URL.revokeObjectURL(url);
        resolve(img);
      };
      img.onerror = () => {
        URL.revokeObjectURL(url);
        reject(new Error("No se pudo decodificar el código QR"));
      };
      img.src = url;
    } catch (err) {
      reject(err);
    }
  });
}

/**
 * Extracts an 8-character uppercase short code from available ticket identifiers.
 * @param {any} entrada 
 * @returns {string}
 */
export function obtenerCodigoCorto(entrada) {
  if (!entrada) return "";
  if (entrada.codigo_corto) return String(entrada.codigo_corto).toUpperCase();
  if (entrada.codigo_qr) return String(entrada.codigo_qr).slice(0, 8).toUpperCase();
  if (entrada.scan_token) return String(entrada.scan_token).replace(/^st_/, "").slice(0, 8).toUpperCase();
  if (entrada.id) return String(entrada.id).replace(/^tix_/, "").slice(0, 8).toUpperCase();
  return "";
}

/**
 * Formats ISO date string to a human-friendly Spanish format for display.
 * @param {string} isoString 
 * @returns {string}
 */
export function formatPublicDateTime(isoString) {
  if (!isoString) return "";
  try {
    const d = new Date(isoString);
    if (isNaN(d.getTime())) return String(isoString);
    return d.toLocaleString("es-PE", {
      weekday: "short",
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return String(isoString);
  }
}

/**
 * Splits text into lines fitting within maxAncho.
 * @param {CanvasRenderingContext2D} ctx 
 * @param {string} texto 
 * @param {number} maxAncho 
 * @returns {string[]}
 */
export function partirLineas(ctx, texto, maxAncho) {
  if (!texto) return [];
  const palabras = String(texto).split(/\s+/);
  const lineas = [];
  let actual = "";
  for (const p of palabras) {
    const prueba = actual ? `${actual} ${p}` : p;
    const width = typeof ctx.measureText === "function" ? ctx.measureText(prueba).width : prueba.length * 20;
    if (actual && width > maxAncho) {
      lineas.push(actual);
      actual = p;
    } else {
      actual = prueba;
    }
  }
  if (actual) lineas.push(actual);
  return lineas;
}

/**
 * Path drawing helper for rounded rectangles.
 * @param {CanvasRenderingContext2D} ctx 
 * @param {number} x 
 * @param {number} y 
 * @param {number} w 
 * @param {number} h 
 * @param {number} r 
 */
export function trazarRedondeado(ctx, x, y, w, h, r) {
  if (typeof ctx.roundRect === "function") {
    ctx.beginPath();
    ctx.roundRect(x, y, w, h, r);
    return;
  }
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}

/**
 * Pure generator: creates a 1080px PNG Blob ticket photo card ready for WhatsApp sharing.
 * Renders Shirley Show brand, on-screen vector QR (via Image+blobURL+drawImage),
 * attendee name, event title, date, venue, ticket type pill, short code, and door instruction.
 *
 * @param {any} entrada 
 * @param {any} [evento] 
 * @returns {Promise<Blob>}
 */
export async function generarFotoEntrada(entrada, evento) {
  if (!entrada) {
    throw new Error("Entrada requerida para generar la foto.");
  }

  if (entrada.estado_pago && entrada.estado_pago !== "Pagado") {
    throw new Error("Esta entrada está pendiente de pago. Debes confirmar el pago antes de generar la imagen.");
  }

  const svgString =
    entrada.qr_svg ||
    entrada.qr ||
    (typeof document !== "undefined" && document.querySelector(".public-ticket-qr-box svg")?.outerHTML);

  if (!svgString) {
    throw new Error("Código QR no disponible para esta entrada.");
  }

  const qrImg = await cargarImagenDesdeSvg(svgString);

  const doc = typeof document !== "undefined" ? document : globalThis.document;
  if (!doc || typeof doc.createElement !== "function") {
    throw new Error("Soporte de DOM/Canvas no disponible.");
  }

  const medidor = doc.createElement("canvas").getContext("2d");
  const maxTexto = ANCHO - (MARGEN * 2);

  const nombreEvento = (evento && evento.nombre) || entrada.nombre_evento || "Evento";
  const fechaHoraStr = (evento && evento.fecha_hora) || entrada.fecha_hora || "";
  const fechaFormateada = formatPublicDateTime(fechaHoraStr);
  const lugar = (evento && evento.lugar) || entrada.lugar || "";
  const comprador = entrada.nombre_comprador || "";
  const tipoEntrada = entrada.tipo_entrada || "General";
  const codigoCorto = obtenerCodigoCorto(entrada);

  const bloques = [
    comprador ? { texto: comprador, font: `700 56px ${FUENTE}`, color: COLOR_TEXTO, alto: 68 } : null,
    nombreEvento ? { texto: nombreEvento, font: `700 46px ${FUENTE}`, color: COLOR_ORO, alto: 58 } : null,
    fechaFormateada ? { texto: fechaFormateada, font: `500 38px ${FUENTE}`, color: COLOR_SUAVE, alto: 50 } : null,
    lugar ? { texto: lugar, font: `500 38px ${FUENTE}`, color: COLOR_SUAVE, alto: 50 } : null,
  ].filter(Boolean);

  for (const b of bloques) {
    medidor.font = b.font;
    b.lineas = partirLineas(medidor, b.texto, maxTexto);
  }

  const altoTextos = bloques.reduce((s, b) => s + b.lineas.length * b.alto, 0);

  // Layout calculation:
  // Top: 70
  // Brand title ("Shirley Show"): 76 + subtitle ("• ENTRADA DIGITAL •"): 24 + gap: 44 = 144
  // QR Frame: MARCO_SIZE (692)
  // Gap: 44
  // Text blocks: altoTextos
  // Gap before pill: 30
  // Ticket type pill: 76
  // Gap: 34
  // Short code line: codigoCorto ? 44 : 0
  // Door reminder: 40
  // Bottom padding: 70
  const alto = 70 + 144 + MARCO_SIZE + 44 + altoTextos + 30 + 76 + 34 + (codigoCorto ? 44 : 0) + 40 + 70;

  const canvas = doc.createElement("canvas");
  canvas.width = ANCHO;
  canvas.height = alto;
  const ctx = canvas.getContext("2d");

  // 1. Background (Warm Ivory)
  ctx.fillStyle = COLOR_BG;
  ctx.fillRect(0, 0, ANCHO, alto);

  // Subtle decorative border for card elegance
  ctx.strokeStyle = COLOR_BORDE;
  ctx.lineWidth = 2;
  trazarRedondeado(ctx, 30, 30, ANCHO - 60, alto - 60, 24);
  ctx.stroke();

  ctx.textAlign = "center";
  ctx.textBaseline = "alphabetic";

  let y = 70;

  // 2. Brand Header: "Shirley Show"
  ctx.font = `700 64px ${FUENTE}`;
  ctx.fillStyle = COLOR_ORO;
  ctx.fillText("Shirley Show", ANCHO / 2, y + 60);
  y += 76;

  ctx.font = `600 24px ${FUENTE}`;
  ctx.fillStyle = COLOR_DIM;
  ctx.fillText("• ENTRADA DIGITAL •", ANCHO / 2, y + 24);
  y += 24 + 44;

  // 3. QR Container Frame with smooth rounded corners
  const xMarco = (ANCHO - MARCO_SIZE) / 2;
  ctx.fillStyle = COLOR_SURFACE;
  trazarRedondeado(ctx, xMarco, y, MARCO_SIZE, MARCO_SIZE, 32);
  ctx.fill();
  ctx.strokeStyle = COLOR_BORDE;
  ctx.lineWidth = 3;
  ctx.stroke();

  // QR Image inside frame (drawn directly from rasterized vector SVG)
  const xQR = (ANCHO - QR_PX) / 2;
  const yQR = y + MARCO_PADDING;
  ctx.drawImage(qrImg, xQR, yQR, QR_PX, QR_PX);
  y += MARCO_SIZE + 44;

  // 4. Details: Buyer, Event, Date, Venue
  for (const b of bloques) {
    ctx.font = b.font;
    ctx.fillStyle = b.color;
    for (const linea of b.lineas) {
      y += b.alto;
      ctx.fillText(linea, ANCHO / 2, y - 14);
    }
  }
  y += 30;

  // 5. Ticket Type Pill (rounded pill with warm ivory-gold background)
  ctx.font = `700 38px ${FUENTE}`;
  const anchoTextoPastilla = typeof ctx.measureText === "function" ? ctx.measureText(tipoEntrada).width : tipoEntrada.length * 25;
  const anchoPastilla = Math.max(anchoTextoPastilla + 100, 240);
  const xPastilla = (ANCHO - anchoPastilla) / 2;

  ctx.fillStyle = COLOR_PASTILLA_BG;
  trazarRedondeado(ctx, xPastilla, y, anchoPastilla, 76, 38);
  ctx.fill();

  ctx.strokeStyle = COLOR_PASTILLA_BORDE;
  ctx.lineWidth = 2.5;
  trazarRedondeado(ctx, xPastilla, y, anchoPastilla, 76, 38);
  ctx.stroke();

  ctx.fillStyle = COLOR_ORO_HOVER;
  ctx.fillText(tipoEntrada, ANCHO / 2, y + 51);
  y += 76 + 34;

  // 6. Short Code & Door instruction
  if (codigoCorto) {
    ctx.font = `600 34px ${FUENTE}`;
    ctx.fillStyle = COLOR_SUAVE;
    ctx.fillText(`Código: ${codigoCorto}`, ANCHO / 2, y + 24);
    y += 44;
  }

  ctx.font = `500 30px ${FUENTE}`;
  ctx.fillStyle = COLOR_DIM;
  ctx.fillText("Muestra este código en la puerta", ANCHO / 2, y + 24);

  return new Promise((resolve, reject) => {
    if (typeof canvas.toBlob === "function") {
      canvas.toBlob((blob) => {
        if (blob) resolve(blob);
        else reject(new Error("No se pudo generar el archivo PNG de la entrada"));
      }, "image/png");
    } else if (typeof canvas.toDataURL === "function") {
      try {
        const dataUrl = canvas.toDataURL("image/png");
        const binStr = atob(dataUrl.split(",")[1]);
        const len = binStr.length;
        const arr = new Uint8Array(len);
        for (let i = 0; i < len; i++) {
          arr[i] = binStr.charCodeAt(i);
        }
        resolve(new Blob([arr], { type: "image/png" }));
      } catch (err) {
        reject(err);
      }
    } else {
      reject(new Error("Canvas no soporta exportación a imagen"));
    }
  });
}

/**
 * Downloads a Blob directly to the client device.
 * @param {Blob} blob 
 * @param {string} nombreArchivo 
 */
export function descargarBlob(blob, nombreArchivo) {
  if (typeof window === "undefined" || typeof document === "undefined") return;
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = nombreArchivo;
  a.style.display = "none";
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  setTimeout(() => URL.revokeObjectURL(url), 10000);
}

/**
 * Orchestrates sharing the ticket photo card via navigator.share with files,
 * falling back to download + notice. Silently ignores AbortError on cancellation.
 * Only intended for paid tickets.
 *
 * @param {any} entrada 
 * @param {any} [evento] 
 * @param {{
 *   onNotice?: (mensaje: string) => void,
 * }} [options={}] 
 * @returns {Promise<void>}
 */
export async function compartirFotoEntrada(entrada, evento, options = {}) {
  const { onNotice } = options;
  const codigo = obtenerCodigoCorto(entrada) || "entrada";
  const nombreArchivo = `entrada-${codigo}.png`;

  let blob;
  try {
    blob = await generarFotoEntrada(entrada, evento);
  } catch (err) {
    if (onNotice) {
      onNotice("No se pudo crear la foto de la entrada.");
    }
    throw err;
  }

  let archivo = null;
  try {
    if (typeof File !== "undefined") {
      archivo = new File([blob], nombreArchivo, { type: "image/png" });
    }
  } catch {
    archivo = null;
  }

  const canShareFiles =
    typeof navigator !== "undefined" &&
    typeof navigator.canShare === "function" &&
    archivo &&
    navigator.canShare({ files: [archivo] });

  if (canShareFiles && typeof navigator.share === "function") {
    try {
      const nombreEvento = (evento && evento.nombre) || entrada.nombre_evento || "el evento";
      const comprador = entrada.nombre_comprador || "";
      await navigator.share({
        title: `Entrada ${nombreEvento}`,
        text: `Entrada para ${nombreEvento} - ${comprador}`,
        files: [archivo],
      });
      return;
    } catch (err) {
      if (err && err.name === "AbortError") {
        // User closed the share menu: silent exit
        return;
      }
      console.warn("Fallo al compartir foto vía navigator.share, usando descarga:", err);
      descargarBlob(blob, nombreArchivo);
      if (onNotice) {
        onNotice("No se pudo abrir el menú de compartir; la foto se guardó en tus descargas.");
      }
      return;
    }
  }

  // Fallback: browser does not support file sharing
  descargarBlob(blob, nombreArchivo);
  if (onNotice) {
    onNotice("La foto se guardó en tus descargas. Adjúntala en el chat de WhatsApp.");
  }
}

/**
 * Injects ticket photo stylesheet link into document head if not already present.
 */
export function ensureTicketPhotoStyles() {
  if (typeof document === "undefined" || !document.head) return;
  if (!document.querySelector("link[href*='ticket-photo.css']")) {
    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = "/assets/styles/events/ticket-photo.css";
    document.head.appendChild(link);
  }
}
