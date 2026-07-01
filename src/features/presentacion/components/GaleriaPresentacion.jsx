import { IconoNota } from './Adornos.jsx'
import { ImagenMedio } from './ImagenMedio.jsx'

// Mosaico de fotos y videos del "Shirley Show y sus amigos". Cada celda abre el lightbox.
export function GaleriaPresentacion({ items, onAbrir }) {
  return (
    <section id="galeria" className="pres__sec pres__sec--crema">
      <div className="pres__wrap">
        <div style={{ textAlign: 'center', marginBottom: 36 }}>
          <span className="pres__eyebrow">Galería</span>
          <h2 className="pres__h2">Shirley Show y sus amigos</h2>
        </div>

        <div className="pres__galeria">
          {items.map((item, i) => (
            <div
              key={item.id}
              className="pres__item"
              style={{ transform: `rotate(${item.rotacion}deg)` }}
              onClick={() => onAbrir(i)}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onAbrir(i) } }}
              aria-label={`Abrir: ${item.titulo}`}
            >
              <div className="pres__item-medio" style={{ aspectRatio: item.aspecto }}>
                <ImagenMedio src={item.poster || item.src} titulo={item.titulo} alt={item.titulo} posicion={item.posicion} />
                {item.esVideo && (
                  <>
                    <div className="pres__video-badge">VIDEO</div>
                    <div className="pres__play"><div className="pres__play-circ"><div className="pres__play-tri" /></div></div>
                  </>
                )}
              </div>
              <div className="pres__item-pie">
                <IconoNota size={12} width={2} style={{ flexShrink: 0 }} />
                <span>{item.titulo}</span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
