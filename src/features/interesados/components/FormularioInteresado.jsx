import { useState } from 'react'
import { Campo } from '../../../components/Campo.jsx'
import { Input } from '../../../components/Input.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { MensajeError } from '../../../components/Estado.jsx'
import { IconoWhatsApp, IconoMessenger } from '../../../components/iconos/Iconos.jsx'

// Formulario para anotar (o editar) una persona interesada en la fase 1.
export function FormularioInteresado({ inicial, guardando, onGuardar, onCancelar }) {
  const [nombre, setNombre] = useState(inicial?.nombre || '')
  const [canal, setCanal] = useState(inicial?.canal || '') // '' | 'WhatsApp' | 'Messenger'
  const [personas, setPersonas] = useState(inicial?.personas || 1)
  const [promotora, setPromotora] = useState(Boolean(inicial?.promotora))
  const [error, setError] = useState('')

  // Toca un canal para elegirlo; toca el mismo de nuevo para dejarlo sin marcar.
  const alternarCanal = (valor) => setCanal((actual) => (actual === valor ? '' : valor))

  function enviar(e) {
    e.preventDefault()
    setError('')
    if (!nombre.trim()) return setError('Escribe el nombre de la persona')
    onGuardar({
      nombre: nombre.trim(),
      canal: canal || null,
      personas: Number(personas) || 1,
      promotora,
    }).catch((err) => setError(err.message))
  }

  return (
    <form className="pila" onSubmit={enviar}>
      <Campo etiqueta="Nombre de la persona" htmlFor="int-nombre">
        <Input id="int-nombre" value={nombre} onChange={(e) => setNombre(e.target.value)} autoFocus />
      </Campo>
      <Campo etiqueta="¿Por dónde lo contactaste? (opcional)">
        <div className="canal-opciones">
          <Boton
            type="button"
            variante={canal === 'WhatsApp' ? 'primario' : 'secundario'}
            onClick={() => alternarCanal('WhatsApp')}
          >
            <IconoWhatsApp size={34} /> WhatsApp
          </Boton>
          <Boton
            type="button"
            variante={canal === 'Messenger' ? 'primario' : 'secundario'}
            onClick={() => alternarCanal('Messenger')}
          >
            <IconoMessenger size={34} /> Messenger
          </Boton>
        </div>
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
      <Campo
        etiqueta="¿Es promotor/a?"
        ayuda="Promete invitar a más gente. Si cumple, no paga entrada pero igual recibe su QR para pasar."
      >
        <div className="alternador">
          <Boton type="button" variante={promotora ? 'primario' : 'secundario'} onClick={() => setPromotora((v) => !v)}>
            {promotora ? 'Sí, es promotor/a' : 'No'}
          </Boton>
        </div>
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
