import './TarjetaMetrica.css'

// Métrica individual (un número grande con su etiqueta). `dinero` reduce el valor
// para que los importes ("S/ 1,250.00") no se desborden de la tarjeta.
export function TarjetaMetrica({ etiqueta, valor, tono = 'neutro', dinero = false }) {
  return (
    <div className={`metrica metrica--${tono} ${dinero ? 'metrica--dinero' : ''}`}>
      <span className="metrica__valor">{valor}</span>
      <span className="metrica__etiqueta">{etiqueta}</span>
    </div>
  )
}
