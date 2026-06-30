// "Disponible para eventos": tres tarjetas con lo que ofrece. Sin precios ni formularios.

const SVG = { fill: 'none', stroke: 'currentColor', strokeWidth: 1.6, strokeLinecap: 'round', strokeLinejoin: 'round' }

const TARJETAS = [
  {
    num: '01',
    titulo: 'Eventos y reuniones',
    texto: 'Para bodas, aniversarios, reuniones familiares y encuentros comunitarios: un repertorio que acompaña cada momento con calidez.',
    icono: (
      <svg width="24" height="24" viewBox="0 0 24 24" {...SVG}><rect x="4" y="5" width="16" height="15" rx="3" /><line x1="4" y1="10" x2="20" y2="10" /><line x1="8" y1="3" x2="8" y2="7" /><line x1="16" y1="3" x2="16" y2="7" /></svg>
    ),
  },
  {
    num: '02',
    titulo: 'Repertorio en vivo',
    texto: 'Canciones interpretadas en vivo, con una voz cercana y arreglos pensados para crear una atmósfera íntima y memorable.',
    icono: (
      <svg width="24" height="24" viewBox="0 0 24 24" {...SVG}><rect x="9" y="3" width="6" height="11" rx="3" /><path d="M6 11a6 6 0 0012 0" /><line x1="12" y1="17" x2="12" y2="21" /><line x1="9" y1="21" x2="15" y2="21" /></svg>
    ),
  },
  {
    num: '03',
    titulo: 'Show con amigos músicos',
    texto: 'Presentaciones junto a otros talentosos músicos invitados, para una experiencia más completa y llena de buena energía.',
    icono: (
      <svg width="24" height="24" viewBox="0 0 24 24" {...SVG}><circle cx="9" cy="9" r="3" /><circle cx="16.5" cy="10.5" r="2.5" /><path d="M4 20c0-3 2.4-5.2 5-5.2s5 2.2 5 5.2" /><path d="M14.5 20c0-2.4 1.7-4.2 3.8-4.2" /></svg>
    ),
  },
]

export function TarjetasEventos() {
  return (
    <section id="eventos" className="pres__sec pres__sec--blanca">
      <div className="pres__wrap" style={{ maxWidth: 1100, textAlign: 'center' }}>
        <span className="pres__eyebrow">Disponible para eventos</span>
        <h2 className="pres__h2">Lleva su música a tu próximo encuentro</h2>
        <div className="pres__cards">
          {TARJETAS.map((t) => (
            <div key={t.num} className="pres__card">
              <span className="pres__card-num">{t.num}</span>
              <div className="pres__card-icono">{t.icono}</div>
              <h3>{t.titulo}</h3>
              <p>{t.texto}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
