import { Tarjeta } from '../../../components/Tarjeta.jsx'
import { Boton } from '../../../components/Boton.jsx'
import { Badge } from '../../../components/Badge.jsx'
import { IconoCheck, IconoLapiz, IconoBasura } from '../../../components/iconos/Iconos.jsx'

// Una persona en la lista de interesados (fase 1), con sus acciones.
// Reutiliza las clases de tarjeta de las entradas (fila__*, entradas-grid).
export function FilaInteresado({ interesado, onConfirmar, onEditar, onEliminar, ocupado }) {
  const confirmado = interesado.estado === 'Confirmado'
  return (
    <Tarjeta>
      <div className="fila__top">
        <strong className="fila__nombre">{interesado.nombre}</strong>
        <span className="fila__tipo">
          {interesado.personas} {interesado.personas === 1 ? 'persona' : 'personas'}
        </span>
      </div>
      <div className="fila__badges">
        <Badge tono={confirmado ? 'exito' : 'primario'}>{confirmado ? 'Ya confirmó' : 'Interesado'}</Badge>
        {interesado.contacto && <span className="fila__medio">{interesado.contacto}</span>}
      </div>
      <div className="fila__acciones">
        {!confirmado && (
          <Boton variante="exito" onClick={() => onConfirmar(interesado)} disabled={ocupado}>
            <IconoCheck size={20} /> Confirmar asistencia
          </Boton>
        )}
        <Boton variante="secundario" onClick={() => onEditar(interesado)} disabled={ocupado}>
          <IconoLapiz size={18} /> Editar
        </Boton>
        <Boton variante="texto" onClick={() => onEliminar(interesado)} disabled={ocupado}>
          <IconoBasura size={18} /> Quitar
        </Boton>
      </div>
    </Tarjeta>
  )
}
