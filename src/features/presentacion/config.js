// Contenido editable de la landing pública.
// Fase C: esto pasará a ser editable desde el panel. Por ahora, fuente única aquí.

// Único canal de contacto: el Facebook de la artista (ella aprueba a sus seguidores).
export const FACEBOOK_URL = 'https://www.facebook.com/share/1Cq5QQcQan'

// Los videos (pesados) se sirven desde un server casero aparte, no desde este servidor
// (así no van al repo ni al volumen de producción). Si ese server pierde conexión, solo
// falla la reproducción del video: la miniatura (`poster`) va empaquetada en la propia web
// (public/), así que siempre se ve.
export const VIDEOS_BASE_URL = 'https://media.shirleyshow.site'

// Etiquetas que aparecen como "chips" bajo el nombre y en la cinta.
export const ETIQUETAS = ['Matrimonios', 'Mariachis', 'Aniversarios', 'Esparcimientos']
export const CINTA = ['Música en vivo', 'Mariachis', 'Esparcimientos', 'Repertorio en vivo']

// Galería del "Shirley Show y sus amigos".
// Fase B: esto se reemplaza por una llamada a GET /api/publico/galeria.
// Por ahora son marcadores (src: null => placeholder elegante) con variedad de formatos
// para que el mosaico se vea bien. Pon `src` con una ruta /media/... cuando tengas las fotos.
// Orden pensado a mano para que los videos (VIDEO badge, más "pesados" visualmente)
// no queden pegados entre sí en el mosaico — se intercalan con fotos.
export const GALERIA = [
  {
    id: 'g2', tipo: 'video', titulo: 'Disfrutando del Show', aspecto: '16 / 9',
    src: `${VIDEOS_BASE_URL}/shirley-show-1.mp4`, poster: '/presentacion/videos/shirley-show-1.jpg',
  },
  {
    id: 'g1', tipo: 'foto', titulo: 'Shirley en escena', aspecto: '4 / 5',
    src: '/presentacion/galeria/shirley-en-escena.jpg', posicion: '100% 30%',
  },
  {
    id: 'g5', tipo: 'video', titulo: 'Un momento especial', aspecto: '16 / 9',
    src: `${VIDEOS_BASE_URL}/momento-especial.mp4`, poster: '/presentacion/videos/momento-especial.jpg',
  },
  { id: 'g3', tipo: 'foto', titulo: 'Baile de Marinera En Vivo', aspecto: '3 / 4', src: '/presentacion/galeria/con-amigos-musicos.jpg' },
  {
    id: 'g6', tipo: 'video', titulo: 'Extracto de Baile', aspecto: '16 / 9',
    src: `${VIDEOS_BASE_URL}/img_1789.mp4`, poster: '/presentacion/videos/cerca-del-publico.jpg',
  },
  {
    id: 'g8', tipo: 'video', titulo: 'Show completo', aspecto: '16 / 9',
    src: `${VIDEOS_BASE_URL}/show-completo.mp4`, poster: '/presentacion/videos/show-completo.jpg',
  },
]

// Pequeñas rotaciones para el efecto "mosaico" de la galería.
export const ROTACIONES = [-2, 1.5, -1, 2, -1.5, 1, -2, 1.5, -1]
