import { IconoNota } from './Adornos.jsx'
import { ImagenMedio } from './ImagenMedio.jsx'

// Bloque de presentación: foto en escena + texto cálido.
export function SobreElla() {
  return (
    <section id="sobre" className="pres__sec pres__sec--crema">
      <IconoNota size={140} width={1} className="pres__nota-fondo" style={{ top: 24, left: -24, transform: 'rotate(-12deg)' }} />
      <div className="pres__wrap pres__about" style={{ maxWidth: 1100, gap: 40 }}>
        <div className="pres__about-foto">
          <div className="pres__about-marco">
            <ImagenMedio src="/presentacion/sobre.png" titulo="Foto de Shirley en escena" alt="Shirley en escena" />
          </div>
          <div className="pres__about-tag">En el escenario</div>
        </div>
        <div className="pres__about-texto">
          <span className="pres__eyebrow">Sobre ella</span>
          <h2 className="pres__h2">Una voz cercana, hecha para acompañarte</h2>
          <p className="pres__lead" style={{ marginTop: 18 }}>
            Shirley es cantante e intérprete de música en vivo, conocida por una voz cálida y un
            estilo cercano que conecta con cada público. Su repertorio combina baladas, música
            popular y momentos acústicos íntimos.
          </p>
          <p className="pres__lead">
            Más que un show, cada presentación es un encuentro: pensado para acompañar tus
            celebraciones familiares y comunitarias con sencillez, calidez y mucho cariño.
          </p>
          <div className="pres__chips" style={{ marginTop: 6 }}>
            <span className="pres__chip pres__chip--blanco">Voz cálida</span>
            <span className="pres__chip pres__chip--blanco">Estilo cercano</span>
            <span className="pres__chip pres__chip--blanco">Repertorio variado</span>
          </div>
        </div>
      </div>
    </section>
  )
}
