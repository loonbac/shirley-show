// Contenido editable de la landing pública.
// Fase C: esto pasará a ser editable desde el panel. Por ahora, fuente única aquí.

// Único canal de contacto: el Facebook de la artista (ella aprueba a sus seguidores).
export const FACEBOOK_URL = 'https://www.facebook.com/share/1Cq5QQcQan'

// Etiquetas que aparecen como "chips" bajo el nombre y en la cinta.
export const ETIQUETAS = ['Matrimonios', 'Mariachis', 'Aniversarios', 'Esparcimientos']
export const CINTA = ['Música en vivo', 'Mariachis', 'Esparcimientos', 'Repertorio en vivo']

// Galería del "Shirley Show y sus amigos".
// Fase B: esto se reemplaza por una llamada a GET /api/publico/galeria.
// Por ahora son marcadores (src: null => placeholder elegante) con variedad de formatos
// para que el mosaico se vea bien. Pon `src` con una ruta /media/... cuando tengas las fotos.
export const GALERIA = [
  { id: 'g1', tipo: 'foto', titulo: 'Shirley en escena', aspecto: '4 / 5', src: null },
  { id: 'g2', tipo: 'video', titulo: 'Presentación junto a la banda', aspecto: '1 / 1', src: null, poster: null },
  { id: 'g3', tipo: 'foto', titulo: 'Con sus amigos músicos', aspecto: '3 / 4', src: null },
  { id: 'g4', tipo: 'foto', titulo: 'Detalle del escenario', aspecto: '1 / 1', src: null },
  { id: 'g5', tipo: 'video', titulo: 'Un momento especial', aspecto: '4 / 3', src: null, poster: null },
  { id: 'g6', tipo: 'foto', titulo: 'Cerca del público', aspecto: '4 / 5', src: null },
  { id: 'g7', tipo: 'foto', titulo: 'Ensayo previo al show', aspecto: '1 / 1', src: null },
  { id: 'g8', tipo: 'video', titulo: 'Show completo', aspecto: '3 / 4', src: null, poster: null },
  { id: 'g9', tipo: 'foto', titulo: 'Backstage con el equipo', aspecto: '4 / 5', src: null },
]

// Pequeñas rotaciones para el efecto "mosaico" de la galería.
export const ROTACIONES = [-2, 1.5, -1, 2, -1.5, 1, -2, 1.5, -1]
