import { useEffect } from 'react'
import { HeroPresentacion } from '../components/HeroPresentacion.jsx'
import { CintaMarquee } from '../components/CintaMarquee.jsx'
import { SobreElla } from '../components/SobreElla.jsx'
import { TarjetasEventos } from '../components/TarjetasEventos.jsx'
import { GaleriaPresentacion } from '../components/GaleriaPresentacion.jsx'
import { LightboxGaleria } from '../components/LightboxGaleria.jsx'
import { ContactoFacebook } from '../components/ContactoFacebook.jsx'
import { PieDePagina } from '../components/PieDePagina.jsx'
import { useGaleria } from '../hooks/useGaleria.js'
import '../Presentacion.css'

// Página pública de presentación de la artista (portada del sitio, sin login).
export default function PresentacionPage() {
  const { items, actual, abrir, cerrar } = useGaleria()

  // Título propio de la cara pública (el panel comparte el index.html).
  useEffect(() => {
    const previo = document.title
    document.title = 'Shirley Show — Música en vivo para tus eventos'
    return () => { document.title = previo }
  }, [])

  return (
    <div className="pres">
      <main>
        {/* Hero + cinta llenan juntos la primera pantalla (la cinta queda visible
            al pie, sin necesidad de hacer scroll). */}
        <div className="pres__portada">
          <HeroPresentacion />
          <CintaMarquee />
        </div>
        <SobreElla />
        <TarjetasEventos />
        <GaleriaPresentacion items={items} onAbrir={abrir} />
        <ContactoFacebook />
      </main>
      <PieDePagina />
      <LightboxGaleria item={actual} onCerrar={cerrar} />
    </div>
  )
}
