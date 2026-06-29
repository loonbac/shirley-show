import { useState } from 'react'
import { useParams } from 'react-router-dom'
import { useEvento } from '../hooks/useEventos.js'
import { CabeceraEvento } from '../components/CabeceraEvento.jsx'
import { ResumenEvento } from '../../resumen/index.js'
import { PanelEntradas, FormularioVenta, entradasApi } from '../../entradas/index.js'
import { Boton } from '../../../components/Boton.jsx'
import { IconoMas } from '../../../components/iconos/Iconos.jsx'
import { Modal } from '../../../components/Modal.jsx'
import { Cargando, MensajeError } from '../../../components/Estado.jsx'
import './DetalleEventoPage.css'

// Detalle del evento: cabecera + resumen + registrar venta + lista de entradas.
// `version` se incrementa tras cada cambio para refrescar resumen y lista a la vez.
export default function DetalleEventoPage() {
  const { id } = useParams()
  const { datos: evento, cargando, error } = useEvento(id)
  const [version, setVersion] = useState(0)
  const [ventaAbierta, setVentaAbierta] = useState(false)
  const [guardando, setGuardando] = useState(false)

  const refrescar = () => setVersion((v) => v + 1)

  async function registrarVenta(datos) {
    setGuardando(true)
    try {
      await entradasApi.registrarVenta(id, datos)
      setVentaAbierta(false)
      refrescar()
    } finally {
      setGuardando(false)
    }
  }

  if (cargando) return <Cargando />
  if (error) return <MensajeError>{error}</MensajeError>
  if (!evento) return null

  return (
    <div className="pila">
      <CabeceraEvento evento={evento} />

      <div className="detalle-cols">
        <aside className="detalle-cols__lado pila">
          <Boton ancho onClick={() => setVentaAbierta(true)}>
            <IconoMas size={22} /> Registrar venta
          </Boton>
          <ResumenEvento eventoId={id} version={version} />
        </aside>
        <div className="detalle-cols__main">
          <PanelEntradas eventoId={id} version={version} onCambio={refrescar} />
        </div>
      </div>

      <Modal abierto={ventaAbierta} titulo="Registrar venta" onCerrar={() => setVentaAbierta(false)}>
        <FormularioVenta evento={evento} guardando={guardando} onRegistrar={registrarVenta} />
      </Modal>
    </div>
  )
}
