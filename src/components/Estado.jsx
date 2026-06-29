import { IconoAlerta } from './iconos/Iconos.jsx'
import './Estado.css'

// Indicador de carga reutilizable.
export function Cargando({ texto = 'Cargando…' }) {
  return (
    <div className="estado">
      <div className="estado__spinner" aria-hidden />
      <p>{texto}</p>
    </div>
  )
}

// Mensaje de error reutilizable.
export function MensajeError({ children }) {
  return (
    <div className="estado estado--error">
      <IconoAlerta size={28} />
      <span>{children}</span>
    </div>
  )
}

// Estado vacío reutilizable (listas sin elementos).
export function Vacio({ children }) {
  return <div className="estado estado--vacio">{children}</div>
}
