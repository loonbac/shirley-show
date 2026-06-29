import { NotasQR } from './NotasQR.jsx'
import { IconoNota } from '../iconos/Iconos.jsx'
import './VisorQR.css'

// QR personalizado de Shirley Show: sus módulos son corcheas con degradado de marca,
// el monograma SS al centro y un marco con motivos musicales (iconos SVG).
// `valor` es el código único de la entrada. `tamano` en píxeles.
export function VisorQR({ valor, tamano = 300 }) {
  return (
    <div className="visorqr">
      <div className="visorqr__titulo">
        <IconoNota size={22} />
        <span className="visorqr__marca">Shirley Show</span>
        <IconoNota size={22} />
      </div>

      <div className="visorqr__marco">
        <NotasQR valor={valor} tamano={tamano} />
      </div>

      <div className="visorqr__pie">
        <IconoNota size={16} /> Entrada musical <IconoNota size={16} />
      </div>
    </div>
  )
}
