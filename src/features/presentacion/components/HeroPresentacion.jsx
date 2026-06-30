import { IconoNota, OndaSonido } from './Adornos.jsx'
import { ImagenMedio } from './ImagenMedio.jsx'
import { ProximoEvento } from './ProximoEvento.jsx'
import { FACEBOOK_URL, ETIQUETAS } from '../config.js'

// Sección principal: foto con halo, logo/nombre, etiquetas y llamadas a la acción.
export function HeroPresentacion() {
  return (
    <section id="inicio" className="pres__sec pres__sec--blanca pres__sec--hero">
      <IconoNota size={220} width={1} className="pres__nota-fondo" style={{ top: 40, right: -30 }} />
      <div className="pres__wrap pres__hero">
        <div className="pres__hero-foto">
          <div className="pres__halo" />
          <svg className="pres__halo-anillo" viewBox="0 0 100 100" fill="none">
            <circle cx="50" cy="50" r="48" stroke="#9a7110" strokeOpacity="0.3" strokeWidth="1" strokeDasharray="1 9" />
          </svg>
          <div className="pres__hero-img">
            <ImagenMedio src="/presentacion/hero.png" titulo="Foto principal de Shirley" alt="Shirley" />
          </div>
          <IconoNota size={26} className="pres__nota-flota" />
          <div className="pres__hero-badge">
            <IconoNota size={13} width={2} />
            <span>Música en vivo</span>
          </div>
        </div>

        <div className="pres__hero-texto">
          <img
            className="pres__hero-logo"
            src="/presentacion/logo.png"
            alt="Shirley Show"
            onError={(e) => { e.currentTarget.src = '/logo.png?v=2' }}
          />
          <p className="pres__hero-sub">Música en vivo para tus eventos</p>
          <div style={{ marginTop: 20 }}>
            <OndaSonido />
          </div>
          <div className="pres__chips">
            {ETIQUETAS.map((t) => <span key={t} className="pres__chip">{t}</span>)}
          </div>
          <div className="pres__buttons">
            <a href={FACEBOOK_URL} target="_blank" rel="noopener noreferrer" className="pres__btn pres__btn--primario">
              <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"><path d="M4 5h16v11H8l-4 4V5z" /></svg>
              Síguela en Facebook
            </a>
            <a href="#galeria" className="pres__btn pres__btn--ghost">Ver galería</a>
          </div>
        </div>

        <ProximoEvento />
      </div>
    </section>
  )
}
