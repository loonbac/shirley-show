import { useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useEvento } from '../hooks/useEventos.js'
import { eventosApi } from '../services/eventosApi.js'
import { CabeceraEvento } from '../components/CabeceraEvento.jsx'
import { ResumenEvento } from '../../resumen/index.js'
import { PanelEntradas, FormularioVenta, entradasApi } from '../../entradas/index.js'
import { PanelInteresados, interesadosApi } from '../../interesados/index.js'
import { PanelGastos } from '../../gastos/index.js'
import { Boton } from '../../../components/Boton.jsx'
import { IconoMas, IconoDinero } from '../../../components/iconos/Iconos.jsx'
import { Modal } from '../../../components/Modal.jsx'
import { Cargando, MensajeError } from '../../../components/Estado.jsx'
import './DetalleEventoPage.css'

// Detalle del evento en 2 fases (pestañas):
//  - Interesados (fase 1): personas contactadas, para estimar la asistencia.
//  - Confirmados (fase 2): venta + QR de quienes ya pagaron.
export default function DetalleEventoPage() {
  const { id } = useParams()
  const navegar = useNavigate()
  const { datos: evento, cargando, error, recargar } = useEvento(id)
  const [version, setVersion] = useState(0)
  const [pestana, setPestana] = useState('interesados')
  const [ventaAbierta, setVentaAbierta] = useState(false)
  const [prefill, setPrefill] = useState(null) // {nombre, contacto, interesadoId} al confirmar de fase 1
  const [guardando, setGuardando] = useState(false)
  const [accionando, setAccionando] = useState(false)
  const [confirmarEliminar, setConfirmarEliminar] = useState(false)

  const refrescar = () => setVersion((v) => v + 1)

  function cerrarVenta() {
    setVentaAbierta(false)
    setPrefill(null)
  }

  // Desde la fase 1: abre la venta en la pestaña Confirmados con el nombre ya puesto.
  function confirmarAsistencia(interesado) {
    setPrefill({ nombre: interesado.nombre, contacto: interesado.contacto || '', interesadoId: interesado.id })
    setPestana('confirmados')
    setVentaAbierta(true)
  }

  async function registrarVenta(datos) {
    setGuardando(true)
    try {
      await entradasApi.registrarVenta(id, datos)
      // Si la venta nació de un interesado de la fase 1, márcalo como confirmado.
      if (prefill?.interesadoId) await interesadosApi.confirmar(prefill.interesadoId)
      cerrarVenta()
      refrescar()
    } finally {
      setGuardando(false)
    }
  }

  async function cambiarFinalizado(finalizar) {
    setAccionando(true)
    try {
      await (finalizar ? eventosApi.finalizar(id) : eventosApi.reabrir(id))
      await recargar()
    } finally {
      setAccionando(false)
    }
  }

  async function eliminar() {
    setAccionando(true)
    try {
      await eventosApi.eliminar(id)
      navegar('/eventos')
    } finally {
      setAccionando(false)
    }
  }

  if (cargando) return <Cargando />
  if (error) return <MensajeError>{error}</MensajeError>
  if (!evento) return null

  return (
    <div className="pila">
      <CabeceraEvento
        evento={evento}
        accionando={accionando}
        onFinalizar={() => cambiarFinalizado(true)}
        onReabrir={() => cambiarFinalizado(false)}
        onEliminar={() => setConfirmarEliminar(true)}
      />

      <div className="pestanas" role="tablist">
        <button
          className={`pestanas__btn${pestana === 'interesados' ? ' pestanas__btn--activa' : ''}`}
          onClick={() => setPestana('interesados')}
        >
          <span className="pestanas__num">1</span> Interesados
        </button>
        <button
          className={`pestanas__btn${pestana === 'confirmados' ? ' pestanas__btn--activa' : ''}`}
          onClick={() => setPestana('confirmados')}
        >
          <span className="pestanas__num">2</span> Confirmados
        </button>
        <button
          className={`pestanas__btn${pestana === 'gastos' ? ' pestanas__btn--activa' : ''}`}
          onClick={() => setPestana('gastos')}
        >
          <IconoDinero size={22} /> Gastos
        </button>
      </div>

      {pestana === 'interesados' && (
        <PanelInteresados eventoId={id} version={version} onConfirmar={confirmarAsistencia} onCambio={refrescar} />
      )}
      {pestana === 'confirmados' && (
        <div className="detalle-cols">
          <aside className="detalle-cols__lado pila">
            <Boton ancho onClick={() => { setPrefill(null); setVentaAbierta(true) }}>
              <IconoMas size={22} /> Registrar venta
            </Boton>
            <ResumenEvento eventoId={id} version={version} />
          </aside>
          <div className="detalle-cols__main">
            <PanelEntradas eventoId={id} version={version} onCambio={refrescar} />
          </div>
        </div>
      )}
      {pestana === 'gastos' && <PanelGastos eventoId={id} version={version} onCambio={refrescar} />}

      <Modal abierto={ventaAbierta} titulo={prefill ? `Confirmar a ${prefill.nombre}` : 'Registrar venta'} onCerrar={cerrarVenta}>
        <FormularioVenta evento={evento} guardando={guardando} onRegistrar={registrarVenta} prefill={prefill} />
      </Modal>

      <Modal abierto={confirmarEliminar} titulo="Eliminar evento" onCerrar={() => setConfirmarEliminar(false)}>
        <div className="pila">
          <p>
            ¿Seguro que quieres eliminar <strong>{evento.nombre}</strong> y todas sus entradas?
            Esta acción no se puede deshacer. Si solo quieres archivarlo, usa <strong>Finalizar</strong>.
          </p>
          <Boton variante="peligro" ancho onClick={eliminar} disabled={accionando}>
            Sí, eliminar definitivamente
          </Boton>
          <Boton variante="secundario" ancho onClick={() => setConfirmarEliminar(false)} disabled={accionando}>
            Cancelar
          </Boton>
        </div>
      </Modal>
    </div>
  )
}
