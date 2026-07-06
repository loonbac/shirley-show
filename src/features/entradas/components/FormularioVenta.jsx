import { useState } from 'react'
import { Campo } from '../../../components/Campo.jsx'
import { Input } from '../../../components/Input.jsx'
import { Select } from '../../../components/Select.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { MensajeError } from '../../../components/Estado.jsx'
import { MEDIOS_PAGO, ESTADOS_PAGO, opciones } from '../constantes.js'
import { soles } from '../../../lib/format.js'
import './FormularioVenta.css'

const MAX_POR_VENTA = 50

// Formulario para registrar una venta. Se elige cuántas entradas de cada tipo
// (ej. 2 General + 1 VIP) y cada una recibe su propio QR.
// `prefill` (opcional) trae nombre/contacto cuando se confirma a un interesado de la fase 1;
// si viene de un promotor/a, ofrece darle su entrada de cortesía (S/ 0, con QR normal).
export function FormularioVenta({ evento, guardando, onRegistrar, prefill }) {
  const tipos = evento.tipos_de_entrada
  const [nombre, setNombre] = useState(prefill?.nombre || '')
  const [contacto, setContacto] = useState(prefill?.contacto || '')
  // Cuántas de cada tipo. Si viene de la fase 1, arranca con las personas que
  // dijo que traería (en el primer tipo); se puede repartir entre tipos a mano.
  const [cantidades, setCantidades] = useState(() =>
    tipos.map((_, i) => (i === 0 ? Math.min(prefill?.personas || 1, MAX_POR_VENTA) : 0))
  )
  const [medio, setMedio] = useState(MEDIOS_PAGO[0])
  const [pago, setPago] = useState('Pagado')
  const [cortesia, setCortesia] = useState(Boolean(prefill?.promotora))
  const [error, setError] = useState('')

  const totalEntradas = cantidades.reduce((s, c) => s + (Number(c) || 0), 0)
  const total = cortesia
    ? 0
    : tipos.reduce((s, t, i) => s + t.precio * (Number(cantidades[i]) || 0), 0)

  function cambiarCantidad(i, valor) {
    const n = Math.max(0, Math.min(MAX_POR_VENTA, Math.floor(Number(valor) || 0)))
    setCantidades((prev) => prev.map((c, j) => (j === i ? n : c)))
  }

  function enviar(e) {
    e.preventDefault()
    setError('')
    if (!nombre.trim()) return setError('Escribe el nombre del comprador')
    const lineas = tipos
      .map((t, i) => ({ tipo_entrada: t.nombre, cantidad: Number(cantidades[i]) || 0 }))
      .filter((l) => l.cantidad > 0)
    if (lineas.length === 0) return setError('Agrega al menos una entrada')
    if (totalEntradas > MAX_POR_VENTA) return setError(`Máximo ${MAX_POR_VENTA} entradas por venta`)
    onRegistrar({
      nombre_comprador: nombre,
      contacto,
      lineas,
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
      <Campo
        etiqueta="¿Cuántas entradas de cada tipo?"
        ayuda="Una por persona (titular + familiares o invitados). Cada una recibe su propio QR."
      >
        <div className="venta-tipos">
          {tipos.map((t, i) => (
            <div key={t.nombre} className="venta-tipo">
              <span className="venta-tipo__nombre">
                {t.nombre} <span className="venta-tipo__precio">{soles(t.precio)}</span>
              </span>
              <div className="venta-tipo__stepper">
                <Boton
                  type="button"
                  variante="secundario"
                  aria-label={`Una entrada ${t.nombre} menos`}
                  onClick={() => cambiarCantidad(i, (Number(cantidades[i]) || 0) - 1)}
                >
                  −
                </Boton>
                <Input
                  type="number"
                  min="0"
                  max={MAX_POR_VENTA}
                  aria-label={`Entradas ${t.nombre}`}
                  value={cantidades[i]}
                  onChange={(e) => cambiarCantidad(i, e.target.value)}
                />
                <Boton
                  type="button"
                  variante="secundario"
                  aria-label={`Una entrada ${t.nombre} más`}
                  onClick={() => cambiarCantidad(i, (Number(cantidades[i]) || 0) + 1)}
                >
                  +
                </Boton>
              </div>
            </div>
          ))}
        </div>
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
        {guardando
          ? 'Registrando…'
          : cortesia
            ? `Dar ${totalEntradas === 1 ? 'entrada' : `${totalEntradas} entradas`} de cortesía · S/ 0.00`
            : `Registrar ${totalEntradas === 1 ? '1 entrada' : `${totalEntradas} entradas`} · ${soles(total)}`}
      </Boton>
    </form>
  )
}
