import { useResumen } from '../hooks/useResumen.js'
import { TarjetaMetrica } from './TarjetaMetrica.jsx'
import { DesglosePagos } from './DesglosePagos.jsx'
import { Cargando, MensajeError } from '../../../components/Estado.jsx'
import { soles } from '../../../lib/format.js'
import './ResumenEvento.css'

// Resumen del evento: vendidas, disponibles, cobrado/pendiente y desglose por medio de pago.
export function ResumenEvento({ eventoId, version }) {
  const { datos: r, cargando, error } = useResumen(eventoId, version)
  if (cargando) return <Cargando />
  if (error) return <MensajeError>{error}</MensajeError>
  if (!r) return null

  return (
    <section className="pila">
      <h2>Resumen</h2>
      <div className="resumen__grid">
        <TarjetaMetrica etiqueta="Vendidas" valor={r.vendidas} tono="primario" />
        <TarjetaMetrica etiqueta="Disponibles" valor={r.disponibles ?? '∞'} />
        <TarjetaMetrica etiqueta="Ingresaron" valor={r.usadas} />
        <TarjetaMetrica etiqueta="Cobrado" valor={soles(r.cobrado)} tono="exito" dinero />
        <TarjetaMetrica etiqueta="Pendiente" valor={soles(r.pendiente)} tono="alerta" dinero />
      </div>
      <DesglosePagos porMedio={r.recaudado_por_medio} />
    </section>
  )
}
