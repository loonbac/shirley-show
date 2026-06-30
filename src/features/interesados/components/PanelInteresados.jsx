import { useState } from 'react'
import { useInteresados } from '../hooks/useInteresados.js'
import { interesadosApi } from '../services/interesadosApi.js'
import { FilaInteresado } from './FilaInteresado.jsx'
import { FormularioInteresado } from './FormularioInteresado.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { Modal } from '../../../components/Modal.jsx'
import { Cargando, MensajeError, Vacio } from '../../../components/Estado.jsx'
import { IconoMas } from '../../../components/iconos/Iconos.jsx'
import '../../entradas/components/FilaEntrada.css' // reutiliza el grid y las clases fila__*
import './PanelInteresados.css'

// Fase 1: lista de personas interesadas + estimación de asistencia para planear el aforo.
export function PanelInteresados({ eventoId, version, onConfirmar, onCambio }) {
  const { datos: lista, cargando, error, recargar } = useInteresados(eventoId, version)
  const [form, setForm] = useState(null) // null | {} (nuevo) | interesado (editar)
  const [aEliminar, setAEliminar] = useState(null)
  const [guardando, setGuardando] = useState(false)
  const [ocupado, setOcupado] = useState(false)

  const interesados = lista || []
  const totalPersonas = interesados.reduce((s, i) => s + (i.personas || 0), 0)
  const confirmados = interesados.filter((i) => i.estado === 'Confirmado').length

  async function guardar(datos) {
    setGuardando(true)
    try {
      if (form?.id) await interesadosApi.actualizar(form.id, datos)
      else await interesadosApi.crear(eventoId, datos)
      setForm(null)
      recargar()
      onCambio?.()
    } finally {
      setGuardando(false)
    }
  }

  async function eliminar() {
    if (!aEliminar) return
    setOcupado(true)
    try {
      await interesadosApi.eliminar(aEliminar.id)
      setAEliminar(null)
      recargar()
      onCambio?.()
    } finally {
      setOcupado(false)
    }
  }

  return (
    <section className="pila">
      <div className="interesados__estimado">
        <span className="interesados__numero">≈ {totalPersonas}</span>
        <span className="interesados__texto">personas vendrían</span>
        <p className="interesados__detalle">
          {interesados.length} {interesados.length === 1 ? 'persona anotada' : 'personas anotadas'} · {confirmados} ya confirmaron
        </p>
      </div>

      <p className="interesados__ayuda">
        Anota a cada persona que te dijo que sí. Con esto sabes cuánta gente vendría para preparar
        las sillas y el espacio. Cuando alguien pague, toca <strong>Confirmar asistencia</strong>.
      </p>

      <Boton ancho onClick={() => setForm({})}>
        <IconoMas size={22} /> Agregar persona
      </Boton>

      {cargando && <Cargando />}
      {error && <MensajeError>{error}</MensajeError>}
      {!cargando && interesados.length === 0 && <Vacio>Aún no hay nadie en la lista. Agrega a la primera persona.</Vacio>}

      <div className="entradas-grid">
        {interesados.map((it) => (
          <FilaInteresado
            key={it.id}
            interesado={it}
            ocupado={ocupado}
            onConfirmar={onConfirmar}
            onEditar={(x) => setForm(x)}
            onEliminar={(x) => setAEliminar(x)}
          />
        ))}
      </div>

      <Modal abierto={Boolean(form)} titulo={form?.id ? 'Editar persona' : 'Agregar persona'} onCerrar={() => setForm(null)}>
        {form && (
          <FormularioInteresado
            inicial={form.id ? form : null}
            guardando={guardando}
            onGuardar={guardar}
            onCancelar={() => setForm(null)}
          />
        )}
      </Modal>

      <Modal abierto={Boolean(aEliminar)} titulo="Quitar persona" onCerrar={() => setAEliminar(null)}>
        <div className="pila">
          <p>
            ¿Quitar a <strong>{aEliminar?.nombre}</strong> de la lista de interesados?
          </p>
          <Boton variante="peligro" ancho onClick={eliminar} disabled={ocupado}>
            Sí, quitar
          </Boton>
          <Boton variante="secundario" ancho onClick={() => setAEliminar(null)} disabled={ocupado}>
            Cancelar
          </Boton>
        </div>
      </Modal>
    </section>
  )
}
