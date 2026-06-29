import './LineaDato.css'

// Línea de dato con icono a la izquierda (icono + texto alineados). Reutilizable.
export function LineaDato({ icono: Icono, children }) {
  return (
    <p className="linea-dato">
      <Icono size={20} />
      <span>{children}</span>
    </p>
  )
}
