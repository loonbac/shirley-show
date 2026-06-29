import { useState } from 'react'
import { Campo } from '../../../components/Campo.jsx'
import { Input } from '../../../components/Input.jsx'
import { Select } from '../../../components/Select.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { MensajeError } from '../../../components/Estado.jsx'
import { MEDIOS_PAGO, ESTADOS_PAGO, opciones } from '../constantes.js'
import { soles } from '../../../lib/format.js'

// Formulario para registrar una venta. Genera `cantidad` entradas (cada una con su QR).
export function FormularioVenta({ evento, guardando, onRegistrar }) {
  const tipos = evento.tipos_de_entrada
  const [nombre, setNombre] = useState('')
  const [contacto, setContacto] = useState('')
  const [tipo, setTipo] = useState(tipos[0]?.nombre || '')
  const [cantidad, setCantidad] = useState(1)
  const [medio, setMedio] = useState(MEDIOS_PAGO[0])
  const [pago, setPago] = useState('Pagado')
  const [error, setError] = useState('')

  const precio = tipos.find((t) => t.nombre === tipo)?.precio || 0
  const total = precio * Number(cantidad || 0)

  function enviar(e) {
    e.preventDefault()
    setError('')
    if (!nombre.trim()) return setError('Escribe el nombre del comprador')
    onRegistrar({
      nombre_comprador: nombre,
      contacto,
      tipo_entrada: tipo,
      cantidad: Number(cantidad),
      medio_pago: medio,
      estado_pago: pago,
    }).catch((err) => setError(err.message))
  }

  return (
    <form className="pila" onSubmit={enviar}>
      <Campo etiqueta="Nombre del comprador" htmlFor="comprador">
        <Input id="comprador" value={nombre} onChange={(e) => setNombre(e.target.value)} autoFocus />
      </Campo>
      <Campo etiqueta="Contacto (WhatsApp)" htmlFor="contacto">
        <Input id="contacto" value={contacto} onChange={(e) => setContacto(e.target.value)} inputMode="tel" />
      </Campo>
      <Campo etiqueta="Tipo de entrada" htmlFor="tipo">
        <Select
          id="tipo"
          value={tipo}
          onChange={(e) => setTipo(e.target.value)}
          opciones={tipos.map((t) => ({ valor: t.nombre, etiqueta: `${t.nombre} — ${soles(t.precio)}` }))}
        />
      </Campo>
      <Campo etiqueta="Cantidad" htmlFor="cantidad" ayuda="Se crea una entrada con QR por cada una.">
        <Input id="cantidad" type="number" min="1" max="50" value={cantidad} onChange={(e) => setCantidad(e.target.value)} />
      </Campo>
      <Campo etiqueta="Medio de pago" htmlFor="medio">
        <Select id="medio" value={medio} onChange={(e) => setMedio(e.target.value)} opciones={opciones(MEDIOS_PAGO)} />
      </Campo>
      <Campo etiqueta="Estado del pago" htmlFor="pago">
        <Select id="pago" value={pago} onChange={(e) => setPago(e.target.value)} opciones={opciones(ESTADOS_PAGO)} />
      </Campo>
      {error && <MensajeError>{error}</MensajeError>}
      <Boton type="submit" ancho disabled={guardando}>
        {guardando ? 'Registrando…' : `Registrar venta · ${soles(total)}`}
      </Boton>
    </form>
  )
}
