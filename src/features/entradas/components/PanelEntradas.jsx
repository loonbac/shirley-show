import { useState } from 'react'
import { useEntradas } from '../hooks/useEntradas.js'
import { entradasApi } from '../services/entradasApi.js'
import { FiltrosEntradas } from './FiltrosEntradas.jsx'
import { FilaEntrada } from './FilaEntrada.jsx'
import { VistaEntrada } from './VistaEntrada.jsx'
import { CompartirEntrada } from './CompartirEntrada.jsx'
import { Modal } from '../../../components/Modal.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { Cargando, MensajeError, Vacio } from '../../../components/Estado.jsx'
import { soles } from '../../../lib/format.js'

// Sección de entradas del detalle: filtros + lista + confirmar pago + ver/compartir + borrar.
// `evento` (nombre, fecha, lugar) se usa en la vista de la entrada y en la foto para WhatsApp.
export function PanelEntradas({ eventoId, evento, version, onCambio }) {
  const [filtros, setFiltros] = useState({ q: '', estado_pago: '' })
  const [verEntrada, setVerEntrada] = useState(null)
  const [aEliminar, setAEliminar] = useState(null)
  const [confirmando, setConfirmando] = useState(false)
  const [borrando, setBorrando] = useState(false)
  const { datos: entradas, cargando, error, recargar } = useEntradas(eventoId, filtros, version)

  async function confirmarPago(entrada) {
    setConfirmando(true)
    try {
      await entradasApi.confirmarPago(entrada.id)
      recargar()
      onCambio?.()
    } finally {
      setConfirmando(false)
    }
  }

  async function eliminar() {
    if (!aEliminar) return
    setBorrando(true)
    try {
      await entradasApi.eliminar(aEliminar.id)
      setAEliminar(null)
      recargar()
      onCambio?.()
    } finally {
      setBorrando(false)
    }
  }

  return (
    <section className="pila">
      <h2>Entradas vendidas</h2>
      <FiltrosEntradas filtros={filtros} onCambiar={setFiltros} />
      {cargando && <Cargando />}
      {error && <MensajeError>{error}</MensajeError>}
      {entradas?.length === 0 && <Vacio>No hay entradas con esos filtros.</Vacio>}
      <div className="entradas-grid">
        {entradas?.map((e) => (
          <FilaEntrada
            key={e.id}
            entrada={e}
            confirmando={confirmando}
            onConfirmarPago={confirmarPago}
            onVer={setVerEntrada}
            onEliminar={setAEliminar}
          />
        ))}
      </div>

      <Modal abierto={Boolean(verEntrada)} titulo="Entrada" onCerrar={() => setVerEntrada(null)}>
        {verEntrada && (
          <div className="pila">
            <VistaEntrada entrada={{ ...verEntrada, evento }} />
            {verEntrada.estado_pago === 'Pagado' && <CompartirEntrada entrada={verEntrada} evento={evento} />}
          </div>
        )}
      </Modal>

      <Modal abierto={Boolean(aEliminar)} titulo="Borrar entrada" onCerrar={() => setAEliminar(null)}>
        <div className="pila">
          <p>
            ¿Borrar la entrada de <strong>{aEliminar?.nombre_comprador}</strong> (
            {aEliminar?.tipo_entrada} · {soles(aEliminar?.precio || 0)})? Se elimina para siempre y
            su QR deja de funcionar. Úsalo solo si la registraste por error.
          </p>
          {aEliminar?.estado_checkin === 'Usada' && (
            <MensajeError>Ojo: esta persona ya ingresó al evento con esta entrada.</MensajeError>
          )}
          <Boton variante="peligro" ancho onClick={eliminar} disabled={borrando}>
            Sí, borrar entrada
          </Boton>
          <Boton variante="secundario" ancho onClick={() => setAEliminar(null)} disabled={borrando}>
            Cancelar
          </Boton>
        </div>
      </Modal>
    </section>
  )
}
