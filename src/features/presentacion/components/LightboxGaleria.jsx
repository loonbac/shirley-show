import { useEffect, useState } from 'react'
import { ImagenMedio } from './ImagenMedio.jsx'

// Los videos viven en un server aparte que puede perder conexión (no es el que sirve la
// página). Si falla la carga, mostramos la miniatura (esa sí siempre está disponible) con
// un aviso, en vez de un reproductor roto.
function VideoConRespaldo({ src, poster, titulo }) {
  const [fallo, setFallo] = useState(false)

  if (fallo) {
    return (
      <div className="pres__lb-video-error">
        <ImagenMedio src={poster} titulo={titulo} alt={titulo} fit="contain" />
        <span className="pres__lb-video-error-msg">El video no está disponible en este momento</span>
      </div>
    )
  }

  return <video src={src} poster={poster || undefined} controls autoPlay playsInline onError={() => setFallo(true)} />
}

// Visor a pantalla completa. Reproduce el video si hay `src`; si no, muestra la foto/placeholder.
export function LightboxGaleria({ item, onCerrar }) {
  const abierto = Boolean(item)

  // Con el visor abierto, la página de atrás no debe poder hacer scroll.
  useEffect(() => {
    if (!abierto) return
    const overflowPrevio = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    const onEsc = (e) => { if (e.key === 'Escape') onCerrar() }
    document.addEventListener('keydown', onEsc)
    return () => {
      document.body.style.overflow = overflowPrevio
      document.removeEventListener('keydown', onEsc)
    }
  }, [abierto, onCerrar])

  if (!item) return null

  return (
    <div className="pres__lb" onClick={onCerrar}>
      <div className="pres__lb-caja" onClick={(e) => e.stopPropagation()}>
        <button className="pres__lb-cerrar" onClick={onCerrar} aria-label="Cerrar">×</button>
        <div className="pres__lb-medio">
          {item.esVideo && item.src ? (
            <VideoConRespaldo key={item.id} src={item.src} poster={item.poster} titulo={item.titulo} />
          ) : (
            <ImagenMedio src={item.src} titulo={item.titulo} alt={item.titulo} fit="contain" />
          )}
        </div>
        <p className="pres__lb-pie">{item.titulo}</p>
      </div>
    </div>
  )
}
