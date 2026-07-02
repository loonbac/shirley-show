import { useState } from 'react'
import { Campo } from '../../../components/Campo.jsx'
import { Input } from '../../../components/Input.jsx'
import { Select } from '../../../components/Select.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { MensajeError } from '../../../components/Estado.jsx'
import { MEDIOS_PAGO, ESTADOS_PAGO, opciones } from '../constantes.js'
import { soles } from '../../../lib/format.js'

// Formulario para registrar una venta. Genera `cantidad` entradas (cada una con su QR).
// `prefill` (opcional) trae nombre/contacto cuando se confirma a un interesado de la fase 1;
// si viene de un promotor/a, ofrece darle su entrada de cortesía (S/ 0, con QR normal).
export function FormularioVenta({ evento, guardando, onRegistrar, prefill }) {
  const tipos = evento.tipos_de_entrada
  const [nombre, setNombre] = useState(prefill?.nombre || '')
  const [contacto, setContacto] = useState(prefill?.contacto || '')
  const [tipo, setTipo] = useState(tipos[0]?.nombre || '')
  const [cantidad, setCantidad] = useState(1)
  const [medio, setMedio] = useState(MEDIOS_PAGO[0])
  const [pago, setPago] = useState('Pagado')
  const [cortesia, setCortesia] = useState(Boolean(prefill?.promotora))
  const [error, setError] = useState('')

  const precio = tipos.find((t) => t.nombre === tipo)?.precio || 0
  const total = cortesia ? 0 : precio * Number(cantidad || 0)

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
      cortesia,
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
      <Campo
        etiqueta="¿Cuántas entradas?"
        htmlFor="cantidad"
        ayuda="Una por persona (titular + familiares o invitados). Cada una recibe su propio QR."
      >
        <Input id="cantidad" type="number" min="1" max="50" value={cantidad} onChange={(e) => setCantidad(e.target.value)} />
      </Campo>
      {prefill?.promotora && (
        <Campo
          etiqueta="Entrada de cortesía (promotor/a)"
          ayuda="Cumplió invitando gente: no paga, pero recibe su QR normal. Desactívalo si esta vez sí paga."
        >
          <div className="alternador">
            <Boton type="button" variante={cortesia ? 'primario' : 'secundario'} onClick={() => setCortesia((v) => !v)}>
              {cortesia ? 'Sí, entra gratis' : 'No, paga su entrada'}
            </Boton>
          </div>
        </Campo>
      )}
      {!cortesia && (
        <Campo etiqueta="Medio de pago" htmlFor="medio">
          <Select id="medio" value={medio} onChange={(e) => setMedio(e.target.value)} opciones={opciones(MEDIOS_PAGO)} />
        </Campo>
      )}
      {!cortesia && (
        <Campo etiqueta="Estado del pago" htmlFor="pago">
          <Select id="pago" value={pago} onChange={(e) => setPago(e.target.value)} opciones={opciones(ESTADOS_PAGO)} />
        </Campo>
      )}
      {error && <MensajeError>{error}</MensajeError>}
      <Boton type="submit" ancho disabled={guardando}>
        {guardando ? 'Registrando…' : cortesia ? 'Dar entrada de cortesía · S/ 0.00' : `Registrar venta · ${soles(total)}`}
      </Boton>
    </form>
  )
}
