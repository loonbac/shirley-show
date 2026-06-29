import { useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { FormularioEvento } from '../components/FormularioEvento.jsx'
import { useEvento } from '../hooks/useEventos.js'
import { eventosApi } from '../services/eventosApi.js'
import { Cargando, MensajeError } from '../../../components/Estado.jsx'

// Sirve tanto para crear (sin id) como para editar (con id) un evento.
export default function FormularioEventoPage() {
  const { id } = useParams()
  const navegar = useNavigate()
  const [guardando, setGuardando] = useState(false)

  const editando = Boolean(id)
  const { datos: evento, cargando, error } = useEvento(id)

  async function guardar(datos) {
    setGuardando(true)
    try {
      const res = editando ? await eventosApi.actualizar(id, datos) : await eventosApi.crear(datos)
      navegar(`/eventos/${res.id}`)
    } finally {
      setGuardando(false)
    }
  }

  if (editando && cargando) return <Cargando />
  if (editando && error) return <MensajeError>{error}</MensajeError>

  return (
    <div className="pila col-angosta">
      <h1>{editando ? 'Editar evento' : 'Crear evento'}</h1>
      <FormularioEvento inicial={editando ? evento : null} guardando={guardando} onGuardar={guardar} />
    </div>
  )
}
