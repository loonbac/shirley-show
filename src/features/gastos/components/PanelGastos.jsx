import { useState } from 'react'
import { useGastos } from '../hooks/useGastos.js'
import { gastosApi } from '../services/gastosApi.js'
import { FilaGasto } from './FilaGasto.jsx'
import { FormularioGasto } from './FormularioGasto.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { Modal } from '../../../components/Modal.jsx'
import { Cargando, MensajeError, Vacio } from '../../../components/Estado.jsx'
import { IconoMas } from '../../../components/iconos/Iconos.jsx'
import { soles } from '../../../lib/format.js'
import '../../entradas/components/FilaEntrada.css' // reutiliza el grid y las clases fila__*
import './PanelGastos.css'

// Sección de gastos del evento: total + desglose por categoría + lista editable.
export function PanelGastos({ eventoId, version, onCambio }) {
  const { datos: lista, cargando, error, recargar } = useGastos(eventoId, version)
  const [form, setForm] = useState(null) // null | {} (nuevo) | gasto (editar)
  const [aEliminar, setAEliminar] = useState(null)
  const [guardando, setGuardando] = useState(false)
  const [ocupado, setOcupado] = useState(false)

  const gastos = lista || []
  const total = gastos.reduce((s, g) => s + (g.monto || 0), 0)

  // Suma por categoría, en el orden en que aparecen.
  const porCategoria = gastos.reduce((acc, g) => {
    acc[g.categoria] = (acc[g.categoria] || 0) + g.monto
    return acc
  }, {})

  async function guardar(datos) {
    setGuardando(true)
    try {
      if (form?.id) await gastosApi.actualizar(form.id, datos)
      else await gastosApi.crear(eventoId, datos)
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
      await gastosApi.eliminar(aEliminar.id)
      setAEliminar(null)
      recargar()
      onCambio?.()
    } finally {
      setOcupado(false)
    }
  }

  return (
    <section className="pila">
      <div className="gastos__total">
        <span className="gastos__texto">Total gastado</span>
        <span className="gastos__numero">{soles(total)}</span>
        {Object.keys(porCategoria).length > 0 && (
          <div className="gastos__desglose">
            {Object.entries(porCategoria).map(([cat, monto]) => (
              <span key={cat} className="gastos__chip">
                {cat}: <strong>{soles(monto)}</strong>
              </span>
            ))}
          </div>
        )}
      </div>

      <Boton ancho onClick={() => setForm({})}>
        <IconoMas size={22} /> Agregar gasto
      </Boton>

      {cargando && <Cargando />}
      {error && <MensajeError>{error}</MensajeError>}
      {!cargando && gastos.length === 0 && <Vacio>Aún no hay gastos anotados.</Vacio>}

      <div className="entradas-grid">
        {gastos.map((g) => (
          <FilaGasto
            key={g.id}
            gasto={g}
            ocupado={ocupado}
            onEditar={(x) => setForm(x)}
            onEliminar={(x) => setAEliminar(x)}
          />
        ))}
      </div>

      <Modal abierto={Boolean(form)} titulo={form?.id ? 'Editar gasto' : 'Agregar gasto'} onCerrar={() => setForm(null)}>
        {form && (
          <FormularioGasto
            inicial={form.id ? form : null}
            guardando={guardando}
            onGuardar={guardar}
            onCancelar={() => setForm(null)}
          />
        )}
      </Modal>

      <Modal abierto={Boolean(aEliminar)} titulo="Quitar gasto" onCerrar={() => setAEliminar(null)}>
        <div className="pila">
          <p>
            ¿Quitar el gasto de <strong>{aEliminar?.categoria}</strong> ({soles(aEliminar?.monto)})?
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
