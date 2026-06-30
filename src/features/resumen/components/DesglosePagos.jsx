import { soles } from '../../../lib/format.js'
import './DesglosePagos.css'

// Recaudado desglosado por medio de pago (Yape / Efectivo).
export function DesglosePagos({ porMedio }) {
  const medios = ['Yape', 'Efectivo']
  return (
    <div className="desglose">
      <h3>Recaudado por medio de pago</h3>
      {medios.map((m) => (
        <div key={m} className="desglose__fila">
          <span>{m}</span>
          <strong>{soles(porMedio[m] || 0)}</strong>
        </div>
      ))}
    </div>
  )
}
