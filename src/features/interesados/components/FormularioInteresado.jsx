import { useState } from 'react'
import { Campo } from '../../../components/Campo.jsx'
import { Input } from '../../../components/Input.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { MensajeError } from '../../../components/Estado.jsx'

// Formulario para anotar (o editar) una persona interesada en la fase 1.
export function FormularioInteresado({ inicial, guardando, onGuardar, onCancelar }) {
  const [nombre, setNombre] = useState(inicial?.nombre || '')
  const [contacto, setContacto] = useState(inicial?.contacto || '')
  const [personas, setPersonas] = useState(inicial?.personas || 1)
  const [error, setError] = useState('')

  function enviar(e) {
    e.preventDefault()
    setError('')
    if (!nombre.trim()) return setError('Escribe el nombre de la persona')
    onGuardar({
      nombre: nombre.trim(),
      contacto: contacto.trim() || null,
      personas: Number(personas) || 1,
    }).catch((err) => setError(err.message))
  }

  return (
    <form className="pila" onSubmit={enviar}>
      <Campo etiqueta="Nombre de la persona" htmlFor="int-nombre">
        <Input id="int-nombre" value={nombre} onChange={(e) => setNombre(e.target.value)} autoFocus />
      </Campo>
      <Campo etiqueta="WhatsApp / contacto (opcional)" htmlFor="int-contacto">
        <Input id="int-contacto" value={contacto} onChange={(e) => setContacto(e.target.value)} inputMode="tel" />
      </Campo>
      <Campo
        etiqueta="¿Cuántas personas vendrían?"
        htmlFor="int-personas"
        ayuda="Contando a familiares o invitados que traería. Sirve para calcular las sillas."
      >
        <Input
          id="int-personas"
          type="number"
          min="1"
          max="100"
          value={personas}
          onChange={(e) => setPersonas(e.target.value)}
        />
      </Campo>
      {error && <MensajeError>{error}</MensajeError>}
      <Boton type="submit" ancho disabled={guardando}>
        {guardando ? 'Guardando…' : 'Guardar'}
      </Boton>
      {onCancelar && (
        <Boton variante="secundario" ancho type="button" onClick={onCancelar} disabled={guardando}>
          Cancelar
        </Boton>
      )}
    </form>
  )
}
