import { useEffect } from 'react'
import { IconoEquis } from './iconos/Iconos.jsx'
import './Modal.css'

// Modal genérico. Se cierra con la X, el fondo o la tecla Escape.
export function Modal({ abierto, titulo, onCerrar, children }) {
  useEffect(() => {
    if (!abierto) return
    const alPresionar = (e) => e.key === 'Escape' && onCerrar()
    window.addEventListener('keydown', alPresionar)
    return () => window.removeEventListener('keydown', alPresionar)
  }, [abierto, onCerrar])

  if (!abierto) return null
  return (
    <div className="modal__fondo" onClick={onCerrar}>
      <div className="modal__caja" onClick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
        <div className="modal__cabecera">
          <h2>{titulo}</h2>
          <button className="modal__cerrar" onClick={onCerrar} aria-label="Cerrar">
            <IconoEquis size={22} />
          </button>
        </div>
        <div className="modal__cuerpo">{children}</div>
      </div>
    </div>
  )
}
