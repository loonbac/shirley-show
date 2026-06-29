import { Boton } from '../../../components/Boton.jsx'
import { IconoWhatsApp, IconoEnlace } from '../../../components/iconos/Iconos.jsx'

// Genera el link público de la entrada y permite enviarlo por WhatsApp o copiarlo.
export function CompartirEntrada({ entrada }) {
  const link = `${window.location.origin}/entrada/${entrada.codigo_qr}`
  const mensaje = `¡Hola ${entrada.nombre_comprador}! Aquí está tu entrada para el evento. Muéstrala en la puerta: ${link}`

  // Si hay contacto, se abre el chat directo (asumiendo Perú +51); si no, el selector de WhatsApp.
  const soloNumero = (entrada.contacto || '').replace(/\D/g, '')
  const destino = soloNumero ? `https://wa.me/51${soloNumero}` : 'https://wa.me/'
  const urlWhatsapp = `${destino}?text=${encodeURIComponent(mensaje)}`

  async function copiar() {
    try {
      await navigator.clipboard.writeText(link)
      alert('Link copiado. Pégalo donde quieras compartirlo.')
    } catch {
      alert(link)
    }
  }

  return (
    <div className="pila">
      <Boton variante="exito" ancho onClick={() => window.open(urlWhatsapp, '_blank')}>
        <IconoWhatsApp size={20} /> Enviar por WhatsApp
      </Boton>
      <Boton variante="secundario" ancho onClick={copiar}>
        <IconoEnlace size={20} /> Copiar link de la entrada
      </Boton>
    </div>
  )
}
