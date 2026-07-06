import { Tarjeta } from '../../../components/Tarjeta.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { Badge, tonoPago, tonoCheckin } from '../../../components/Badge.jsx'
import { IconoCheck, IconoTicket, IconoBasura } from '../../../components/iconos/Iconos.jsx'
import './FilaEntrada.css'

// Una entrada en la lista, con sus estados y acciones (confirmar pago / ver entrada / borrar).
export function FilaEntrada({ entrada, confirmando, onConfirmarPago, onVer, onEliminar }) {
  return (
    <Tarjeta>
      <div className="fila__top">
        <strong className="fila__nombre">{entrada.nombre_comprador}</strong>
        <span className="fila__tipo">{entrada.tipo_entrada}</span>
      </div>
      <div className="fila__badges">
        <Badge tono={tonoPago(entrada.estado_pago)}>{entrada.estado_pago}</Badge>
        <Badge tono={tonoCheckin(entrada.estado_checkin)}>
          {entrada.estado_checkin === 'Usada' ? 'Ingresó' : 'No ingresó'}
        </Badge>
        <span className="fila__medio">{entrada.medio_pago}</span>
      </div>
      <div className="fila__acciones">
        {entrada.estado_pago === 'Pendiente' && (
          <Boton variante="exito" onClick={() => onConfirmarPago(entrada)} disabled={confirmando}>
            <IconoCheck size={20} /> Confirmar pago
          </Boton>
        )}
        <Boton variante="secundario" onClick={() => onVer(entrada)}>
          <IconoTicket size={20} /> Ver / compartir
        </Boton>
        <Boton variante="texto" onClick={() => onEliminar(entrada)}>
          <IconoBasura size={18} /> Borrar
        </Boton>
      </div>
    </Tarjeta>
  )
}
