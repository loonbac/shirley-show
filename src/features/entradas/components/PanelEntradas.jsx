import { useState } from 'react'
import { useEntradas } from '../hooks/useEntradas.js'
import { entradasApi } from '../services/entradasApi.js'
import { FiltrosEntradas } from './FiltrosEntradas.jsx'
import { FilaEntrada } from './FilaEntrada.jsx'
import { VistaEntrada } from './VistaEntrada.jsx'
import { CompartirEntrada } from './CompartirEntrada.jsx'
import { Modal } from '../../../components/Modal.jsx'
import { Cargando, MensajeError, Vacio } from '../../../components/Estado.jsx'

// Sección de entradas del detalle: filtros + lista + confirmar pago + ver/compartir.
export function PanelEntradas({ eventoId, version, onCambio }) {
  const [filtros, setFiltros] = useState({ q: '', estado_pago: '' })
  const [verEntrada, setVerEntrada] = useState(null)
  const [confirmando, setConfirmando] = useState(false)
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
          />
        ))}
      </div>

      <Modal abierto={Boolean(verEntrada)} titulo="Entrada" onCerrar={() => setVerEntrada(null)}>
        {verEntrada && (
          <div className="pila">
            <VistaEntrada entrada={verEntrada} />
            {verEntrada.estado_pago === 'Pagado' && <CompartirEntrada entrada={verEntrada} />}
          </div>
        )}
      </Modal>
    </section>
  )
}
