import './Tarjeta.css'

// Tarjeta contenedora reutilizable. Si recibe onClick se vuelve interactiva.
export function Tarjeta({ children, onClick, className = '' }) {
  const clases = `tarjeta ${onClick ? 'tarjeta--click' : ''} ${className}`
  if (onClick) {
    return (
      <button type="button" className={clases} onClick={onClick}>
        {children}
      </button>
    )
  }
  return <div className={clases}>{children}</div>
}
