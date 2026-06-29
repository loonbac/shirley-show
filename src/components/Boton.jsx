import './Boton.css'

// Botón reutilizable. variante: 'primario' | 'secundario' | 'peligro' | 'exito' | 'texto'.
export function Boton({ variante = 'primario', ancho, children, className = '', ...props }) {
  const clases = ['boton', `boton--${variante}`, ancho ? 'boton--ancho' : '', className].filter(Boolean).join(' ')
  return (
    <button className={clases} {...props}>
      {children}
    </button>
  )
}
