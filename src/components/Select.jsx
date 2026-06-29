// Select genérico. opciones: [{ valor, etiqueta }]. Usa el estilo compartido `.control`.
export function Select({ opciones = [], className = '', ...props }) {
  return (
    <select className={`control ${className}`} {...props}>
      {opciones.map((o) => (
        <option key={o.valor} value={o.valor}>
          {o.etiqueta}
        </option>
      ))}
    </select>
  )
}
