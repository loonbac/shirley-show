import './TarjetaMetrica.css'

// Métrica individual (un número grande con su etiqueta).
export function TarjetaMetrica({ etiqueta, valor, tono = 'neutro' }) {
  return (
    <div className={`metrica metrica--${tono}`}>
      <span className="metrica__valor">{valor}</span>
      <span className="metrica__etiqueta">{etiqueta}</span>
    </div>
  )
}
