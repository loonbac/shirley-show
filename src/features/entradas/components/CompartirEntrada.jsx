import { useState } from 'react'
import { Boton } from '../../../components/Boton.jsx'
import { IconoWhatsApp, IconoEnlace, IconoCamara } from '../../../components/iconos/Iconos.jsx'
import { generarFotoEntrada } from '../lib/fotoEntrada.js'
import './CompartirEntrada.css'

// Comparte la entrada. Lo principal es la FOTO (imagen con QR y datos) para mandarla
// por WhatsApp; el link público queda como opción secundaria.
export function CompartirEntrada({ entrada, evento }) {
  const [generando, setGenerando] = useState(false)
  const [aviso, setAviso] = useState('')

  const link = `${window.location.origin}/entrada/${entrada.codigo_qr}`
  const mensaje = `¡Hola ${entrada.nombre_comprador}! Aquí está tu entrada para el evento. Muéstrala en la puerta: ${link}`

  // Si hay contacto, se abre el chat directo (asumiendo Perú +51); si no, el selector de WhatsApp.
  const soloNumero = (entrada.contacto || '').replace(/\D/g, '')
  const destino = soloNumero ? `https://wa.me/51${soloNumero}` : 'https://wa.me/'
  const urlWhatsapp = `${destino}?text=${encodeURIComponent(mensaje)}`

  async function enviarFoto() {
    setGenerando(true)
    setAviso('')
    try {
      const blob = await generarFotoEntrada(entrada, evento)
      const nombreArchivo = `entrada-${entrada.codigo_qr.slice(0, 8)}.png`
      const archivo = new File([blob], nombreArchivo, { type: 'image/png' })
      if (navigator.canShare?.({ files: [archivo] })) {
        // Abre el menú de compartir del celular: ahí eliges WhatsApp y el chat.
        await navigator.share({ files: [archivo] })
      } else {
        descargar(blob, nombreArchivo)
        setAviso('La foto se guardó en tus descargas. Adjúntala en el chat de WhatsApp.')
      }
    } catch (e) {
      if (e?.name === 'AbortError') return // cerró el menú de compartir, no es un error
      console.error('No se pudo compartir la foto de la entrada:', e)
      // Si compartir falló, al menos deja la foto descargada.
      try {
        const blob = await generarFotoEntrada(entrada, evento)
        descargar(blob, `entrada-${entrada.codigo_qr.slice(0, 8)}.png`)
        setAviso('No se pudo abrir el menú de compartir; la foto se guardó en tus descargas.')
      } catch {
        setAviso('No se pudo crear la foto. Puedes enviar el link de abajo.')
      }
    } finally {
      setGenerando(false)
    }
  }

  async function copiar() {
    try {
      await navigator.clipboard.writeText(link)
      setAviso('Link copiado. Pégalo donde quieras compartirlo.')
    } catch {
      alert(link)
    }
  }

  return (
    <div className="pila">
      <Boton variante="exito" ancho onClick={enviarFoto} disabled={generando}>
        <IconoCamara size={20} /> {generando ? 'Creando foto…' : 'Enviar foto por WhatsApp'}
      </Boton>
      <Boton variante="secundario" ancho onClick={() => window.open(urlWhatsapp, '_blank')}>
        <IconoWhatsApp size={20} /> Enviar solo el link
      </Boton>
      <Boton variante="texto" ancho onClick={copiar}>
        <IconoEnlace size={20} /> Copiar link
      </Boton>
      {aviso && <p className="compartir__aviso">{aviso}</p>}
    </div>
  )
}

function descargar(blob, nombre) {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = nombre
  a.click()
  setTimeout(() => URL.revokeObjectURL(url), 5000)
}
