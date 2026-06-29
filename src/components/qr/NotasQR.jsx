import { useMemo } from 'react'
import { generarMatriz, esEsquinaDeteccion } from './generarMatriz.js'
import { Nota, Finder } from './piezasQr.jsx'
import { emblemaDataUrl } from './emblema.js'

const MARGEN = 2 // zona de silencio (módulos) alrededor del QR

// QR cuyos módulos de datos son corcheas, con degradado de marca y el monograma SS al centro.
export function NotasQR({ valor, tamano = 240 }) {
  const { size, notas, lienzo, logo } = useMemo(() => construir(valor), [valor])

  return (
    <svg width={tamano} height={tamano} viewBox={`0 0 ${lienzo} ${lienzo}`} role="img" aria-label="Código QR de la entrada">
      <defs>
        <linearGradient id="gradNota" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2={lienzo} y2={lienzo}>
          <stop offset="0" stopColor="#7c3aed" />
          <stop offset="1" stopColor="#c026d3" />
        </linearGradient>
      </defs>
      <rect x="0" y="0" width={lienzo} height={lienzo} rx="2" fill="#ffffff" />

      {/* Esquinas de detección */}
      <Finder r0={MARGEN} c0={MARGEN} />
      <Finder r0={MARGEN} c0={MARGEN + size - 7} />
      <Finder r0={MARGEN + size - 7} c0={MARGEN} />

      {/* Módulos de datos como corcheas */}
      {notas.map((n) => (
        <Nota key={n.k} x={n.x} y={n.y} />
      ))}

      {/* Emblema musical al centro, sobre un fondo blanco limpio */}
      <rect x={logo.o} y={logo.o} width={logo.s} height={logo.s} rx={logo.s * 0.24} fill="#ffffff" />
      <image href={emblemaDataUrl} x={logo.o} y={logo.o} width={logo.s} height={logo.s} />
    </svg>
  )
}

// Calcula el lienzo, las notas a dibujar (excluye esquinas y centro del logo) y el recuadro del logo.
function construir(valor) {
  const { size, dark } = generarMatriz(valor)
  const lienzo = size + MARGEN * 2

  const logoMods = Math.round(size * 0.32)
  const desde = Math.floor((size - logoMods) / 2)
  const hasta = desde + logoMods
  const enLogo = (r, c) => r >= desde && r < hasta && c >= desde && c < hasta

  const notas = []
  for (let r = 0; r < size; r++) {
    for (let c = 0; c < size; c++) {
      if (!dark(r, c)) continue
      if (esEsquinaDeteccion(r, c, size) || enLogo(r, c)) continue
      notas.push({ k: `${r}-${c}`, x: c + MARGEN, y: r + MARGEN })
    }
  }
  const logo = { o: desde + MARGEN, s: logoMods }
  return { size, notas, lienzo, logo }
}
