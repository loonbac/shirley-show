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
	prot("GET", "/api/eventos/{id}/resumen", s.resumen)
	prot("GET", "/api/eventos/{id}/entradas", s.listarEntradas)
	prot("POST", "/api/eventos/{id}/entradas", s.registrarVenta)
	prot("POST", "/api/eventos/{id}/checkin", s.checkin)
	prot("GET", "/api/entradas/{id}", s.obtenerEntrada)
	prot("POST", "/api/entradas/{id}/confirmar-pago", s.confirmarPago)
	prot("POST", "/api/entradas/{id}/enviar-whatsapp", s.enviarWhatsApp)
	prot("GET", "/api/whatsapp/estado", s.whatsappEstado)
	prot("POST", "/api/whatsapp/conectar", s.whatsappConectar)

	// Frontend compilado (SPA)
	mux.HandleFunc("/", s.servirSPA)

	return cabecerasSeguridad(cors(mux))
}

// cabecerasSeguridad agrega cabeceras defensivas a todas las respuestas.
func cabecerasSeguridad(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("X-Content-Type-Options", "nosniff")       // no adivinar tipos MIME
		w.Header().Set("X-Frame-Options", "DENY")                 // anti clickjacking
		w.Header().Set("Referrer-Policy", "no-referrer")          // no filtrar URLs
		next.ServeHTTP(w, r)
	})
}

// health responde 200 para indicar que el servidor está listo para recibir tráfico.
func (s *Server) health(w http.ResponseWriter, r *http.Request) {
	escribirJSON(w, http.StatusOK, map[string]bool{"ok": true})
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

// cors agrega cabeceras CORS y responde los preflight OPTIONS.
func cors(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
}
