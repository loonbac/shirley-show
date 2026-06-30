import { IconoNota } from './Adornos.jsx'
import { FACEBOOK_URL } from '../config.js'

// Cierre con una única llamada a la acción: escribirle por Facebook (su único canal).
export function ContactoFacebook() {
  return (
    <section id="contacto" className="pres__sec pres__contacto">
      <IconoNota size={160} width={1} className="pres__nota-fondo" style={{ bottom: -10, left: -20, opacity: 0.06 }} />
      <div className="pres__contacto-wrap">
        <h2>¿Quieres tenerla en tu próximo evento?</h2>
        <p>Escríbele por Facebook y cuéntale sobre tu evento — ella te responde con mucho gusto.</p>
        <a href={FACEBOOK_URL} target="_blank" rel="noopener noreferrer" className="pres__btn pres__btn--primario pres__btn--xl pres__cta">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"><path d="M4 5h16v11H8l-4 4V5z" /></svg>
          Escríbele por Facebook
        </a>
      </div>
    </section>
  )
}
