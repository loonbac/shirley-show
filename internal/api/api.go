// Package api expone la API HTTP y sirve el frontend compilado.
package api

import (
	"errors"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"shirleyshow/internal/auth"
	"shirleyshow/internal/config"
	"shirleyshow/internal/store"
	"shirleyshow/internal/whatsapp"
)

type Server struct {
	st      *store.Store
	cfg     config.Config
	wa      *whatsapp.Cliente // puede ser nil si WhatsApp está desactivado
	limiter *limitadorLogin
}

func New(st *store.Store, cfg config.Config, wa *whatsapp.Cliente) *Server {
	return &Server{st: st, cfg: cfg, wa: wa, limiter: nuevoLimitador()}
}

type handlerErr func(http.ResponseWriter, *http.Request) error

// h envuelve un handler que devuelve error y lo traduce a respuesta HTTP.
func (s *Server) h(fn handlerErr) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if err := fn(w, r); err != nil {
			var ae apiError
			switch {
			case errors.As(err, &ae):
				errorJSON(w, ae.status, ae.msg)
			case errors.Is(err, store.ErrNoEncontrado):
				errorJSON(w, http.StatusNotFound, "No encontrado")
			default:
				log.Printf("error interno: %v", err)
				errorJSON(w, http.StatusInternalServerError, "Error del servidor")
			}
		}
	}
}

// auth protege rutas: exige un token de sesión válido.
func (s *Server) authMW(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		h := r.Header.Get("Authorization")
		if !strings.HasPrefix(h, "Bearer ") {
			errorJSON(w, http.StatusUnauthorized, "Falta el token de sesión")
			return
		}
		if _, err := auth.VerificarToken(s.cfg.JWTSecret, strings.TrimPrefix(h, "Bearer ")); err != nil {
			errorJSON(w, http.StatusUnauthorized, "Sesión inválida o expirada")
			return
		}
		next.ServeHTTP(w, r)
	})
}

// Handler arma el router completo (API protegida/pública + frontend estático).
func (s *Server) Handler() http.Handler {
	mux := http.NewServeMux()

	// Salud (para el health check de Coolify/Docker; pública y sin estado)
	mux.HandleFunc("GET /api/health", s.health)

	// robots.txt: la página es privada; pedimos a los buscadores que no la rastreen.
	mux.HandleFunc("GET /robots.txt", s.robots)

	// Afiches subidos. Pública para que las etiquetas <img> carguen sin token
	// (el navegador no manda Authorization en una imagen); la web vive tras el túnel.
	mux.HandleFunc("GET /media/{archivo}", s.servirMedia)

	// Redirección del QR: un escáner común que abra .../r/{token} cae al Facebook
	// de la artista. Nuestro escáner (Modo puerta) en cambio resuelve el token al
	// código de la entrada. Pública por diseño.
	mux.HandleFunc("GET /r/{token}", s.redirigirQR)

	// Públicas
	mux.HandleFunc("POST /api/auth/login", s.h(s.login))
	mux.HandleFunc("GET /api/publico/entrada/{codigo}", s.h(s.entradaPublica))

	// Protegidas
	prot := func(method, pat string, fn handlerErr) {
		mux.Handle(method+" "+pat, s.authMW(s.h(fn)))
	}
	prot("GET", "/api/eventos", s.listarEventos)
	prot("POST", "/api/eventos", s.crearEvento)
	prot("GET", "/api/eventos/{id}", s.obtenerEvento)
	prot("PUT", "/api/eventos/{id}", s.actualizarEvento)
	prot("DELETE", "/api/eventos/{id}", s.eliminarEvento)
	prot("POST", "/api/eventos/{id}/finalizar", s.finalizarEvento)
	prot("POST", "/api/eventos/{id}/reabrir", s.reabrirEvento)
	prot("POST", "/api/eventos/{id}/imagen", s.subirImagen)
	prot("DELETE", "/api/eventos/{id}/imagen", s.quitarImagen)
	prot("GET", "/api/eventos/{id}/resumen", s.resumen)
	prot("GET", "/api/eventos/{id}/entradas", s.listarEntradas)
	prot("POST", "/api/eventos/{id}/entradas", s.registrarVenta)
	prot("POST", "/api/eventos/{id}/checkin", s.checkin)
	// Fase 1 (interesados / invitación)
	prot("GET", "/api/eventos/{id}/interesados", s.listarInteresados)
	prot("POST", "/api/eventos/{id}/interesados", s.crearInteresado)
	prot("PUT", "/api/interesados/{id}", s.actualizarInteresado)
	prot("POST", "/api/interesados/{id}/confirmar", s.confirmarInteresado)
	prot("DELETE", "/api/interesados/{id}", s.eliminarInteresado)
	prot("GET", "/api/entradas/{id}", s.obtenerEntrada)
	prot("POST", "/api/entradas/{id}/confirmar-pago", s.confirmarPago)
	prot("POST", "/api/entradas/{id}/enviar-whatsapp", s.enviarWhatsApp)
	prot("GET", "/api/whatsapp/estado", s.whatsappEstado)
	prot("POST", "/api/whatsapp/conectar", s.whatsappConectar)

	// Frontend compilado (SPA)
	mux.HandleFunc("/", s.servirSPA)

	return cabecerasSeguridad(s.cors(mux))
}

// cabecerasSeguridad agrega cabeceras defensivas a todas las respuestas.
func cabecerasSeguridad(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("X-Content-Type-Options", "nosniff") // no adivinar tipos MIME
		w.Header().Set("X-Frame-Options", "DENY")           // anti clickjacking
		w.Header().Set("Referrer-Policy", "no-referrer")    // no filtrar URLs
		// Página privada: que los buscadores no la indexen ni sigan enlaces ni
		// cacheen una copia. Es lo más fiable (los bots serios respetan este header).
		w.Header().Set("X-Robots-Tag", "noindex, nofollow, noarchive")
		// HSTS solo cuando la conexión llega por HTTPS (terminado en Cloudflare):
		// obliga al navegador a usar siempre HTTPS para el dominio.
		if esHTTPS(r) {
			w.Header().Set("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
		}
		next.ServeHTTP(w, r)
	})
}

// esHTTPS detecta si la petición original llegó por HTTPS, ya sea directamente
// (TLS local) o terminada por un proxy/túnel (Cloudflare manda X-Forwarded-Proto).
func esHTTPS(r *http.Request) bool {
	return r.TLS != nil || strings.EqualFold(r.Header.Get("X-Forwarded-Proto"), "https")
}

// baseURL devuelve el origen público (PUBLIC_BASE_URL si está, o el de la petición).
func (s *Server) baseURL(r *http.Request) string {
	if b := strings.TrimRight(s.cfg.PublicBaseURL, "/"); b != "" {
		return b
	}
	esquema := "http"
	if esHTTPS(r) {
		esquema = "https"
	}
	return esquema + "://" + r.Host
}

// ponerQR arma la URL que codifica el QR de la entrada: baseURL/r/<token>. Un escáner
// común la abre y cae a Facebook; el nuestro extrae el token. Si no hay token, deja vacío
// (el frontend cae al código).
func (s *Server) ponerQR(r *http.Request, e *store.Entrada) {
	if e != nil && e.ScanToken != "" {
		e.QR = s.baseURL(r) + "/r/" + e.ScanToken
	}
}

// redirigirQR manda cualquier visita a /r/{token} al Facebook de la artista.
func (s *Server) redirigirQR(w http.ResponseWriter, r *http.Request) {
	destino := s.cfg.FacebookURL
	if destino == "" {
		destino = "/"
	}
	http.Redirect(w, r, destino, http.StatusFound)
}

// health responde 200 para indicar que el servidor está listo para recibir tráfico.
func (s *Server) health(w http.ResponseWriter, r *http.Request) {
	escribirJSON(w, http.StatusOK, map[string]bool{"ok": true})
}

// robots sirve un robots.txt que prohíbe el rastreo de todo el sitio (página privada).
func (s *Server) robots(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/plain; charset=utf-8")
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write([]byte("User-agent: *\nDisallow: /\n"))
}

// servirSPA entrega archivos de dist/ y, si no existe el archivo, devuelve index.html
// (para que el enrutado del lado del cliente funcione).
func (s *Server) servirSPA(w http.ResponseWriter, r *http.Request) {
	limpio := filepath.Clean(r.URL.Path)
	archivo := filepath.Join(s.cfg.WebDir, limpio)
	if info, err := os.Stat(archivo); err == nil && !info.IsDir() {
		http.ServeFile(w, r, archivo)
		return
	}
	http.ServeFile(w, r, filepath.Join(s.cfg.WebDir, "index.html"))
}

// cors agrega cabeceras CORS y responde los preflight OPTIONS. Solo emite cabeceras
// cross-origin si CORS_ORIGIN está configurado; por defecto ("") la API solo acepta
// peticiones del mismo origen (la SPA se sirve desde el propio backend en producción).
func (s *Server) cors(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if origen := s.cfg.CORSOrigin; origen != "" {
			w.Header().Set("Access-Control-Allow-Origin", origen)
			w.Header().Set("Vary", "Origin")
			w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")
			w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
		}
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
}
