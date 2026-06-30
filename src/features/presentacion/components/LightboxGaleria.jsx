import { useEffect } from 'react'
import { ImagenMedio } from './ImagenMedio.jsx'

// Visor a pantalla completa. Reproduce el video si hay `src`; si no, muestra la foto/placeholder.
export function LightboxGaleria({ item, onCerrar }) {
  useEffect(() => {
    if (!item) return
    const onEsc = (e) => { if (e.key === 'Escape') onCerrar() }
    document.addEventListener('keydown', onEsc)
    return () => document.removeEventListener('keydown', onEsc)
  }, [item, onCerrar])

  if (!item) return null

  return (
    <div className="pres__lb" onClick={onCerrar}>
      <div className="pres__lb-caja" onClick={(e) => e.stopPropagation()}>
        <button className="pres__lb-cerrar" onClick={onCerrar} aria-label="Cerrar">×</button>
        <div className="pres__lb-medio">
          {item.esVideo && item.src ? (
            <video src={item.src} poster={item.poster || undefined} controls autoPlay playsInline />
          ) : (
            <ImagenMedio src={item.src} titulo={item.titulo} alt={item.titulo} fit="contain" />
          )}
        </div>
        <p className="pres__lb-pie">{item.titulo}</p>
      </div>
    </div>
  )
}
