import { useState, useCallback } from 'react'
import { GALERIA, ROTACIONES } from '../config.js'

// Estado de la galería + lightbox.
// Fase B: cambiar `items` por datos traídos de GET /api/publico/galeria.
export function useGaleria() {
  const [abiertoEn, setAbiertoEn] = useState(null)

  const items = GALERIA.map((item, i) => ({
    ...item,
    esVideo: item.tipo === 'video',
    rotacion: ROTACIONES[i % ROTACIONES.length],
  }))

  const abrir = useCallback((i) => setAbiertoEn(i), [])
  const cerrar = useCallback(() => setAbiertoEn(null), [])

  const actual = abiertoEn != null ? items[abiertoEn] : null

  return { items, actual, abrir, cerrar }
}
