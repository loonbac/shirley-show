import { Input } from '../../../components/Input.jsx'
import { Select } from '../../../components/Select.jsx'
import './FiltrosEntradas.css'

// Filtros de la lista de entradas: búsqueda por nombre y estado de pago.
export function FiltrosEntradas({ filtros, onCambiar }) {
  const set = (campo) => (e) => onCambiar({ ...filtros, [campo]: e.target.value })
  return (
    <div className="filtros">
      <Input placeholder="Buscar por nombre…" value={filtros.q} onChange={set('q')} />
      <Select
        value={filtros.estado_pago}
        onChange={set('estado_pago')}
        opciones={[
          { valor: '', etiqueta: 'Todos los pagos' },
          { valor: 'Pagado', etiqueta: 'Pagado' },
          { valor: 'Pendiente', etiqueta: 'Pendiente' },
        ]}
      />
    </div>
  )
}
