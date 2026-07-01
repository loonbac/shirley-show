import { useState } from 'react'
import { IconoNota } from './Adornos.jsx'

// Imagen con respaldo: si no hay `src` o falla la carga, muestra un placeholder
// elegante (degradado + nota + título) en lugar de una imagen rota.
// Útil mientras no haya fotos reales (Fase A) y para fallos de red.
export function ImagenMedio({ src, alt, titulo, fit = 'cover', posicion }) {
  const [falló, setFalló] = useState(false)

  if (!src || falló) {
    return (
      <div className="pres__ph" aria-label={alt || titulo}>
        <IconoNota size={26} width={1.6} />
        {titulo && <span>{titulo}</span>}
      </div>
    )
  }

  return (
    <img
      src={src}
      alt={alt || titulo || ''}
      loading="lazy"
      onError={() => setFalló(true)}
      style={{ objectFit: fit, objectPosition: posicion }}
    />
  )
}
