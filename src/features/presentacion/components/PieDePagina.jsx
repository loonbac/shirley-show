import { OndaSonido } from './Adornos.jsx'

// Pie sobrio: onda de sonido, marca y año.
export function PieDePagina() {
  const año = new Date().getFullYear()
  return (
    <footer className="pres__footer">
      <div style={{ marginBottom: 14 }}>
        <OndaSonido
          alturas={[7, 12, 17, 11, 15, 8]}
          ancho={2.5}
          opacidades={[0.4, 0.5, 0.65, 0.5, 0.6, 0.4]}
        />
      </div>
      <div className="pres__footer-marca">Shirley Show</div>
      <div className="pres__footer-copy">© {año} Shirley Show · Música en vivo</div>
    </footer>
  )
}
