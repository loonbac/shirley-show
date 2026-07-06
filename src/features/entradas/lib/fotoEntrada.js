import { construir } from '../../../components/qr/NotasQR.jsx'
import { fechaLarga } from '../../../lib/format.js'

// Genera una FOTO (PNG) de la entrada, lista para mandar por WhatsApp como imagen:
// logo de la marca, el QR musical y los datos del evento. Se dibuja en un canvas
// reutilizando el mismo componente NotasQR del visor (mismo QR que en pantalla).

const ANCHO = 1080
const MARGEN = 70
const QR_PX = 660

// Colores de marca (mismos valores que tokens.css; el canvas no lee variables CSS).
const COLOR_TEXTO = '#1f1a2e'
const COLOR_SUAVE = '#5b5470'
const COLOR_PRIMARIO_OSCURO = '#6024c9'
const COLOR_PRIMARIO_SUAVE = '#ede7fb'
const COLOR_BORDE = '#d8d2e6'

export async function generarFotoEntrada(entrada, evento) {
  const [logoQR, logoImg] = await Promise.all([
    cargarImagen('/logo-qr.png').catch(() => null), // sin logo el QR sigue siendo válido
    cargarImagen('/logo.png').catch(() => null),
  ])

  // Primer paso: medir cuánto ocupa cada texto para darle al canvas la altura justa.
  const medidor = document.createElement('canvas').getContext('2d')
  const maxTexto = ANCHO - MARGEN * 2
  const bloques = [
    { texto: entrada.nombre_comprador, font: `700 58px ${FUENTE}`, color: COLOR_TEXTO, alto: 68 },
    evento && { texto: evento.nombre, font: `700 46px ${FUENTE}`, color: COLOR_PRIMARIO_OSCURO, alto: 56 },
    evento && { texto: fechaLarga(evento.fecha_hora), font: `500 38px ${FUENTE}`, color: COLOR_SUAVE, alto: 50 },
    evento?.lugar && { texto: evento.lugar, font: `500 38px ${FUENTE}`, color: COLOR_SUAVE, alto: 50 },
  ].filter(Boolean)
  for (const b of bloques) {
    medidor.font = b.font
    b.lineas = partirLineas(medidor, b.texto, maxTexto)
  }

  const altoLogo = logoImg ? 150 : 90
  const altoTextos = bloques.reduce((s, b) => s + b.lineas.length * b.alto, 0)
  const alto = 60 + altoLogo + 40 + (QR_PX + 56) + 44 + altoTextos + 26 + 76 + 30 + 40 + 70

  const canvas = document.createElement('canvas')
  canvas.width = ANCHO
  canvas.height = alto
  const ctx = canvas.getContext('2d')

  ctx.fillStyle = '#ffffff'
  ctx.fillRect(0, 0, ANCHO, alto)
  ctx.textAlign = 'center'
  ctx.textBaseline = 'alphabetic'
  let y = 60

  // Logo de la marca arriba (si no carga, el nombre en texto)
  if (logoImg) {
    const h = altoLogo
    const w = (logoImg.width / logoImg.height) * h
    ctx.drawImage(logoImg, (ANCHO - w) / 2, y, w, h)
  } else {
    ctx.font = `700 64px ${FUENTE}`
    ctx.fillStyle = COLOR_PRIMARIO_OSCURO
    ctx.fillText('Shirley Show', ANCHO / 2, y + 70)
  }
  y += altoLogo + 40

  // QR dentro de un marco redondeado
  const marco = QR_PX + 56
  const xMarco = (ANCHO - marco) / 2
  ctx.strokeStyle = COLOR_BORDE
  ctx.lineWidth = 3
  trazarRedondeado(ctx, xMarco, y, marco, marco, 28)
  ctx.stroke()
  dibujarQR(ctx, entrada.qr || entrada.codigo_qr, (ANCHO - QR_PX) / 2, y + 28, QR_PX, logoQR)
  y += marco + 44

  // Datos: comprador, evento, fecha, lugar
  for (const b of bloques) {
    ctx.font = b.font
    ctx.fillStyle = b.color
    for (const linea of b.lineas) {
      y += b.alto
      ctx.fillText(linea, ANCHO / 2, y - 14)
    }
  }
  y += 26

  // Tipo de entrada como pastilla
  ctx.font = `700 38px ${FUENTE}`
  const anchoPastilla = ctx.measureText(entrada.tipo_entrada).width + 90
  ctx.fillStyle = COLOR_PRIMARIO_SUAVE
  trazarRedondeado(ctx, (ANCHO - anchoPastilla) / 2, y, anchoPastilla, 76, 38)
  ctx.fill()
  ctx.fillStyle = COLOR_PRIMARIO_OSCURO
  ctx.fillText(entrada.tipo_entrada, ANCHO / 2, y + 51)
  y += 76 + 30

  // Código corto y recordatorio
  ctx.font = `500 32px ${FUENTE}`
  ctx.fillStyle = COLOR_SUAVE
  ctx.fillText(`Código: ${entrada.codigo_qr.slice(0, 8).toUpperCase()}`, ANCHO / 2, y + 20)
  ctx.fillText('Muestra este código en la puerta', ANCHO / 2, y + 62)

  return new Promise((resolver, rechazar) => {
    canvas.toBlob((blob) => (blob ? resolver(blob) : rechazar(new Error('No se pudo crear la imagen'))), 'image/png')
  })
}

const FUENTE = "system-ui, 'Segoe UI', Roboto, sans-serif"

// Dibuja el mismo QR musical de NotasQR pero en canvas: puntitos con degradado
// de marca, esquinas de detección sólidas y el logo al centro. Usa el mismo
// `construir` del componente, así la foto y la pantalla nunca se desincronizan.
function dibujarQR(ctx, valor, x, y, px, logoQR) {
  const { size, notas, lienzo, logo } = construir(valor)
  const margen = (lienzo - size) / 2

  ctx.save()
  ctx.translate(x, y)
  ctx.scale(px / lienzo, px / lienzo)

  const grad = ctx.createLinearGradient(0, 0, lienzo, lienzo)
  grad.addColorStop(0, '#0f1b2e')
  grad.addColorStop(1, '#c0246f')

  ctx.fillStyle = '#ffffff'
  ctx.fillRect(0, 0, lienzo, lienzo)

  // Esquinas de detección (misma geometría que <Finder/>)
  for (const [r0, c0] of [
    [margen, margen],
    [margen, margen + size - 7],
    [margen + size - 7, margen],
  ]) {
    ctx.fillStyle = grad
    trazarRedondeado(ctx, c0 + 0.1, r0 + 0.1, 6.8, 6.8, 1.8)
    ctx.fill()
    ctx.fillStyle = '#ffffff'
    trazarRedondeado(ctx, c0 + 1.1, r0 + 1.1, 4.8, 4.8, 1.2)
    ctx.fill()
    ctx.fillStyle = grad
    trazarRedondeado(ctx, c0 + 2, r0 + 2, 3, 3, 0.9)
    ctx.fill()
  }

  // Módulos de datos como puntitos (mismo radio que <Punto/>)
  ctx.fillStyle = grad
  ctx.beginPath()
  for (const n of notas) {
    ctx.moveTo(n.x + 0.92, n.y + 0.5)
    ctx.arc(n.x + 0.5, n.y + 0.5, 0.42, 0, Math.PI * 2)
  }
  ctx.fill()

  // Logo de marca al centro, encajado en su recuadro (como preserveAspectRatio meet)
  if (logoQR) {
    const escala = Math.min(logo.s / logoQR.width, logo.s / logoQR.height)
    const w = logoQR.width * escala
    const h = logoQR.height * escala
    ctx.drawImage(logoQR, logo.o + (logo.s - w) / 2, logo.o + (logo.s - h) / 2, w, h)
  }
  ctx.restore()
}

function cargarImagen(src) {
  return new Promise((resolver, rechazar) => {
    const img = new Image()
    img.onload = () => resolver(img)
    img.onerror = () => rechazar(new Error('No se pudo cargar la imagen'))
    img.src = src
  })
}

// Parte un texto en líneas que quepan en `maxAncho` (según la fuente ya puesta en ctx).
function partirLineas(ctx, texto, maxAncho) {
  const palabras = String(texto).split(/\s+/)
  const lineas = []
  let actual = ''
  for (const p of palabras) {
    const prueba = actual ? `${actual} ${p}` : p
    if (actual && ctx.measureText(prueba).width > maxAncho) {
      lineas.push(actual)
      actual = p
    } else {
      actual = prueba
    }
  }
  if (actual) lineas.push(actual)
  return lineas
}

// Rectángulo redondeado a mano (ctx.roundRect aún no está en todos los navegadores).
function trazarRedondeado(ctx, x, y, w, h, r) {
  ctx.beginPath()
  ctx.moveTo(x + r, y)
  ctx.arcTo(x + w, y, x + w, y + h, r)
  ctx.arcTo(x + w, y + h, x, y + h, r)
  ctx.arcTo(x, y + h, x, y, r)
  ctx.arcTo(x, y, x + w, y, r)
  ctx.closePath()
}
