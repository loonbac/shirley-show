import './Campo.css'

// Envuelve una etiqueta + control de formulario, con texto grande y legible.
export function Campo({ etiqueta, htmlFor, ayuda, children }) {
  return (
    <label className="campo" htmlFor={htmlFor}>
      <span className="campo__etiqueta">{etiqueta}</span>
      {children}
      {ayuda && <span className="campo__ayuda">{ayuda}</span>}
    </label>
  )
}
