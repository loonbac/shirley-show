import { useMemo } from 'react'
import { generarMatriz, esEsquinaDeteccion } from './generarMatriz.js'
import { Punto, Finder } from './piezasQr.jsx'

const MARGEN = 2 // zona de silencio (módulos) alrededor del QR
const LOGO = '/logo-qr.png' // logo de marca (recortado, sin margen) al centro del QR

// QR cuyos módulos de datos son corcheas, con el logo de la marca grande al centro.
export function NotasQR({ valor, tamano = 240 }) {
  const { size, notas, lienzo, logo } = useMemo(() => construir(valor), [valor])

  return (
    <svg width={tamano} height={tamano} viewBox={`0 0 ${lienzo} ${lienzo}`} role="img" aria-label="Código QR de la entrada">
      <defs>
        <linearGradient id="gradNota" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2={lienzo} y2={lienzo}>
          {/* Colores del logo: azul marino del badge -> magenta de "y sus amigos" (ambos oscuros, escanean bien) */}
          <stop offset="0" stopColor="#0f1b2e" />
          <stop offset="1" stopColor="#c0246f" />
        </linearGradient>
      </defs>
      <rect x="0" y="0" width={lienzo} height={lienzo} rx="2" fill="#ffffff" />

      {/* Esquinas de detección */}
      <Finder r0={MARGEN} c0={MARGEN} />
      <Finder r0={MARGEN} c0={MARGEN + size - 7} />
      <Finder r0={MARGEN + size - 7} c0={MARGEN} />

      {/* Módulos de datos como puntitos */}
      {notas.map((n) => (
        <Punto key={n.k} x={n.x} y={n.y} />
      ))}

      {/* Logo de marca grande al centro, encima de las notas y SIN recuadro: su fondo
          es transparente, así los puntos se ven hasta el borde del logo (sin caja blanca). */}
      <image
        href={LOGO}
        x={logo.o}
        y={logo.o}
        width={logo.s}
        height={logo.s}
        preserveAspectRatio="xMidYMid meet"
      />
    </svg>
  )
}

// Calcula el lienzo, las notas a dibujar (excluye esquinas y centro del logo) y el recuadro del logo.
// Exportado para que la foto de la entrada (canvas) dibuje exactamente el mismo QR.
export function construir(valor) {
  const { size, dark } = generarMatriz(valor)
  const lienzo = size + MARGEN * 2

  const logoMods = Math.round(size * 0.5)
  const desde = Math.floor((size - logoMods) / 2)

  // Dibujamos TODAS las notas (también bajo el centro): el logo va encima y su fondo
  // transparente deja ver los puntos hasta su borde. Solo se omiten las esquinas.
  const notas = []
  for (let r = 0; r < size; r++) {
    for (let c = 0; c < size; c++) {
      if (!dark(r, c)) continue
      if (esEsquinaDeteccion(r, c, size)) continue
      notas.push({ k: `${r}-${c}`, x: c + MARGEN, y: r + MARGEN })
    }
  }
  const logo = { o: desde + MARGEN, s: logoMods }
  return { size, notas, lienzo, logo }
}
