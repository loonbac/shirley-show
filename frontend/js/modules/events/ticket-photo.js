/**
 * ticket-photo.js - Generador de foto PNG de entrada para WhatsApp (1080px)
 * Replicación fiel de fotoEntrada.js de Shirley Show.
 *
 * Incluye:
 * - Logo superior de marca (o texto de respaldo)
 * - QR musical con degradado #0f1b2e -> #c0246f, esquinas redondeadas y logo-qr central
 * - Marco redondeado alrededor del QR
 * - Nombre del comprador (58px, negrita)
 * - Nombre del evento (46px, morado #6024c9)
 * - Fecha larga y lugar (38px, gris suave)
 * - Pastilla de tipo de entrada (fondo lavanda suave #ede7fb, texto morado #6024c9)
 * - Código corto (primeros 8 caracteres en MAYÚSCULAS)
 * - Recordatorio: "Muestra este código en la puerta"
 */

import { dibujarQR, trazarRedondeado } from "./ticket-qr-dots.js";

const ANCHO = 1080;
const MARGEN = 70;
const QR_PX = 660;

// Colores de marca originales (fotoEntrada.js / tokens Shirley Show)
const COLOR_TEXTO = "#1f1a2e";
const COLOR_SUAVE = "#5b5470";
const COLOR_PRIMARIO_OSCURO = "#6024c9";
const COLOR_PRIMARIO_SUAVE = "#ede7fb";
const COLOR_BORDE = "#d8d2e6";

const FUENTE = "system-ui, 'Segoe UI', Roboto, sans-serif";

/**
 * Formatea una fecha ISO a fecha larga en español (Perú).
 * Coincide exactamente con fechaLarga() de lib/format.js original.
 * @param {string} iso 
 * @returns {string}
 */
export function fechaLarga(iso) {
  if (!iso) return "";
  try {
    const d = new Date(iso);
    if (isNaN(d.getTime())) return String(iso);
    return d.toLocaleString("es-PE", {
      weekday: "long",
      day: "numeric",
      month: "long",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return String(iso);
  }
}

/**
 * Extrae el código corto en MAYÚSCULAS (primeros 8 caracteres).
 * Prioriza codigo_qr, tolerando scan_token o id si el DTO no expone el secreto en crudo.
 * @param {any} entrada 
 * @returns {string}
 */
export function obtenerCodigoCorto(entrada) {
  if (!entrada) return "";
  if (entrada.codigo_qr) return String(entrada.codigo_qr).slice(0, 8).toUpperCase();
  if (entrada.codigo_corto) return String(entrada.codigo_corto).toUpperCase();
  if (entrada.scan_token) return String(entrada.scan_token).replace(/^st_/, "").slice(0, 8).toUpperCase();
  if (entrada.id) return String(entrada.id).replace(/^tix_/, "").slice(0, 8).toUpperCase();
  return "";
}

/**
 * Carga asíncronamente un elemento HTMLImageElement desde una URL.
 * @param {string} src 
 * @returns {Promise<HTMLImageElement>}
 */
export function cargarImagen(src) {
  return new Promise((resolver, rechazar) => {
    if (typeof Image === "undefined") {
      return rechazar(new Error("Image no disponible en este entorno"));
    }
    const img = new Image();
    img.onload = () => resolver(img);
    img.onerror = () => rechazar(new Error(`No se pudo cargar la imagen: ${src}`));
    img.src = src;
  });
}

/**
 * Intenta cargar una imagen probando una lista de rutas candidatas con fallback.
 * @param {string[]} rutas 
 * @returns {Promise<HTMLImageElement|null>}
 */
export async function cargarImagenConFallback(rutas) {
  for (const src of rutas) {
    try {
      return await cargarImagen(src);
    } catch {}
  }
  return null;
}

/**
 * Parte un texto en líneas que quepan en maxAncho según la tipografía activa.
 * @param {CanvasRenderingContext2D|null} ctx 
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
    const width = (ctx && typeof ctx.measureText === "function")
      ? ctx.measureText(prueba).width
      : prueba.length * 24;
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
 * Genera una FOTO (PNG) de la entrada a 1080px de ancho, fiel a fotoEntrada.js original.
 *
 * @param {any} entrada Datos de la entrada (nombre_comprador, tipo_entrada, codigo_qr, etc.)
 * @param {any} [evento] Datos del evento (nombre, fecha_hora, lugar)
 * @returns {Promise<Blob>} Blob con imagen PNG lista para compartir o descargar
 */
export async function generarFotoEntrada(entrada, evento) {
  if (!entrada) {
    throw new Error("Entrada requerida para generar la foto.");
  }

  if (entrada.estado_pago && entrada.estado_pago !== "Pagado") {
    throw new Error("Esta entrada está pendiente de pago. Debes confirmar el pago antes de generar la imagen.");
  }

  // Valor a codificar en el QR: el mismo que valida la puerta (codigo_qr)
  const valorQR =
    entrada.codigo_qr ||
    entrada.qr ||
    (typeof entrada.qr_svg === "string" ? entrada.qr_svg : "") ||
    entrada.scan_token ||
    entrada.id ||
    "";

  if (!valorQR) {
    throw new Error("Código QR no disponible para esta entrada.");
  }

  const [logoQR, logoImg] = await Promise.all([
    cargarImagenConFallback([
      "/assets/public/logo-qr.png",
      "/public/logo-qr.png",
      "/logo-qr.png",
    ]).catch(() => null),
    cargarImagenConFallback([
      "/assets/public/logo.png",
      "/public/logo.png",
      "/logo.png",
    ]).catch(() => null),
  ]);

  const doc = typeof document !== "undefined" ? document : globalThis.document;
  if (!doc || typeof doc.createElement !== "function") {
    throw new Error("Soporte de DOM/Canvas no disponible.");
  }

  // Medir cuánto ocupa cada texto para darle al canvas la altura justa
  const canvasMedidor = doc.createElement("canvas");
  const medidor = canvasMedidor.getContext ? canvasMedidor.getContext("2d") : null;
  const maxTexto = ANCHO - MARGEN * 2;

  const nombreComprador = entrada.nombre_comprador || "";
  const nombreEvento = (evento && evento.nombre) || entrada.nombre_evento || "";
  const fechaHoraStr = (evento && evento.fecha_hora) || entrada.fecha_hora || "";
  const fechaTexto = fechaHoraStr ? fechaLarga(fechaHoraStr) : "";
  const lugar = (evento && evento.lugar) || entrada.lugar || "";

  const bloques = [
    nombreComprador ? { texto: nombreComprador, font: `700 58px ${FUENTE}`, color: COLOR_TEXTO, alto: 68 } : null,
    nombreEvento ? { texto: nombreEvento, font: `700 46px ${FUENTE}`, color: COLOR_PRIMARIO_OSCURO, alto: 56 } : null,
    fechaTexto ? { texto: fechaTexto, font: `500 38px ${FUENTE}`, color: COLOR_SUAVE, alto: 50 } : null,
    lugar ? { texto: lugar, font: `500 38px ${FUENTE}`, color: COLOR_SUAVE, alto: 50 } : null,
  ].filter(Boolean);

  for (const b of bloques) {
    if (medidor) medidor.font = b.font;
    b.lineas = partirLineas(medidor, b.texto, maxTexto);
  }

  const altoLogo = logoImg ? 150 : 90;
  const altoTextos = bloques.reduce((s, b) => s + b.lineas.length * b.alto, 0);
  const alto = 60 + altoLogo + 40 + (QR_PX + 56) + 44 + altoTextos + 26 + 76 + 30 + 40 + 70;

  const canvas = doc.createElement("canvas");
  canvas.width = ANCHO;
  canvas.height = alto;
  const ctx = canvas.getContext("2d");

  // Fondo blanco puro
  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, ANCHO, alto);
  ctx.textAlign = "center";
  ctx.textBaseline = "alphabetic";
  let y = 60;

  // 1. Logo de la marca arriba (si no carga, el nombre en texto)
  if (logoImg && logoImg.width && logoImg.height) {
    const h = altoLogo;
    const w = (logoImg.width / logoImg.height) * h;
    ctx.drawImage(logoImg, (ANCHO - w) / 2, y, w, h);
  } else {
    ctx.font = `700 64px ${FUENTE}`;
    ctx.fillStyle = COLOR_PRIMARIO_OSCURO;
    ctx.fillText("Shirley Show", ANCHO / 2, y + 70);
  }
  y += altoLogo + 40;

  // 2. QR musical dentro de marco redondeado
  const marco = QR_PX + 56;
  const xMarco = (ANCHO - marco) / 2;
  ctx.strokeStyle = COLOR_BORDE;
  ctx.lineWidth = 3;
  trazarRedondeado(ctx, xMarco, y, marco, marco, 28);
  ctx.stroke();
  dibujarQR(ctx, valorQR, (ANCHO - QR_PX) / 2, y + 28, QR_PX, logoQR);
  y += marco + 44;

  // 3. Datos: comprador, evento, fecha, lugar
  for (const b of bloques) {
    ctx.font = b.font;
    ctx.fillStyle = b.color;
    for (const linea of b.lineas) {
      y += b.alto;
      ctx.fillText(linea, ANCHO / 2, y - 14);
    }
  }
  y += 26;

  // 4. Tipo de entrada como pastilla morada suave
  const tipoEntrada = entrada.tipo_entrada || "General";
  ctx.font = `700 38px ${FUENTE}`;
  const anchoTextoPastilla = (ctx && typeof ctx.measureText === "function")
    ? ctx.measureText(tipoEntrada).width
    : tipoEntrada.length * 24;
  const anchoPastilla = anchoTextoPastilla + 90;
  ctx.fillStyle = COLOR_PRIMARIO_SUAVE;
  trazarRedondeado(ctx, (ANCHO - anchoPastilla) / 2, y, anchoPastilla, 76, 38);
  ctx.fill();
  ctx.fillStyle = COLOR_PRIMARIO_OSCURO;
  ctx.fillText(tipoEntrada, ANCHO / 2, y + 51);
  y += 76 + 30;

  // 5. Código corto y recordatorio
  const codigoCorto = obtenerCodigoCorto(entrada);
  ctx.font = `500 32px ${FUENTE}`;
  ctx.fillStyle = COLOR_SUAVE;
  ctx.fillText(`Código: ${codigoCorto}`, ANCHO / 2, y + 20);
  ctx.fillText("Muestra este código en la puerta", ANCHO / 2, y + 62);

  return new Promise((resolver, rechazar) => {
    if (typeof canvas.toBlob === "function") {
      canvas.toBlob(
        (blob) => (blob ? resolver(blob) : rechazar(new Error("No se pudo crear la imagen"))),
        "image/png"
      );
    } else if (typeof canvas.toDataURL === "function") {
      try {
        const dataUrl = canvas.toDataURL("image/png");
        const binStr = atob(dataUrl.split(",")[1]);
        const len = binStr.length;
        const arr = new Uint8Array(len);
        for (let i = 0; i < len; i++) {
          arr[i] = binStr.charCodeAt(i);
        }
        resolver(new Blob([arr], { type: "image/png" }));
      } catch (err) {
        rechazar(err);
      }
    } else {
      rechazar(new Error("No se pudo crear la imagen"));
    }
  });
}

/**
 * Descarga directamente un Blob en el dispositivo del usuario.
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
 * Comparte la foto de entrada por navigator.share o recurre a descarga directa.
 * @param {any} entrada 
 * @param {any} [evento] 
 * @param {{ onNotice?: (mensaje: string) => void }} [options={}] 
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

  descargarBlob(blob, nombreArchivo);
  if (onNotice) {
    onNotice("La foto se guardó en tus descargas.");
  }
}

/**
 * Inyecta los estilos de la foto de entrada en el head si no existen.
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

export { trazarRedondeado };
