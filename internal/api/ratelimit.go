package api

import (
	"net"
	"net/http"
	"strings"
	"sync"
	"time"
)

// limitadorLogin frena ataques de fuerza bruta: tras varios fallos desde una IP,
// bloquea nuevos intentos por un tiempo. En memoria (la app corre en una sola instancia).
type limitadorLogin struct {
	mu        sync.Mutex
	porClave  map[string]*intentos
	maxFallos int
	bloqueo   time.Duration
}

type intentos struct {
	fallos int
	hasta  time.Time
}

func nuevoLimitador() *limitadorLogin {
	return &limitadorLogin{
		porClave:  make(map[string]*intentos),
		maxFallos: 5,
		bloqueo:   15 * time.Minute,
	}
}

// permitido indica si la clave (IP) puede intentar login ahora.
func (l *limitadorLogin) permitido(clave string) bool {
	l.mu.Lock()
	defer l.mu.Unlock()
	r := l.porClave[clave]
	return r == nil || time.Now().After(r.hasta)
}

// fallo registra un intento fallido y bloquea al superar el máximo.
func (l *limitadorLogin) fallo(clave string) {
	l.mu.Lock()
	defer l.mu.Unlock()
	r := l.porClave[clave]
	if r == nil {
		r = &intentos{}
		l.porClave[clave] = r
	}
	r.fallos++
	if r.fallos >= l.maxFallos {
		r.hasta = time.Now().Add(l.bloqueo)
		r.fallos = 0
	}
	l.podar()
}

// exito limpia el registro tras un login correcto.
func (l *limitadorLogin) exito(clave string) {
	l.mu.Lock()
	defer l.mu.Unlock()
	delete(l.porClave, clave)
}

// podar evita crecimiento ilimitado del mapa (llamar bajo lock).
func (l *limitadorLogin) podar() {
	if len(l.porClave) < 1000 {
		return
	}
	now := time.Now()
	for k, r := range l.porClave {
		if r.fallos == 0 && now.After(r.hasta) {
			delete(l.porClave, k)
		}
	}
}

// ipCliente obtiene la IP real del cliente. Solo cree en las cabeceras CF-Connecting-IP
// y X-Forwarded-For si la conexión directa proviene de una red de confianza (el túnel /
// proxy). Si el origen es directo y no confiable, usa la IP real del socket; así un
// atacante no puede falsificar esas cabeceras para evadir el rate-limit de login.
func ipCliente(r *http.Request, confiables []*net.IPNet) string {
	directo := r.RemoteAddr
	if host, _, err := net.SplitHostPort(directo); err == nil {
		directo = host
	}
	if ipDeConfianza(directo, confiables) {
		if cf := r.Header.Get("CF-Connecting-IP"); cf != "" {
			return strings.TrimSpace(cf)
		}
		if xff := r.Header.Get("X-Forwarded-For"); xff != "" {
			return strings.TrimSpace(strings.Split(xff, ",")[0])
		}
	}
	return directo
}

// ipDeConfianza indica si la IP pertenece a alguna de las redes confiables.
func ipDeConfianza(ip string, confiables []*net.IPNet) bool {
	p := net.ParseIP(ip)
	if p == nil {
		return false
	}
	for _, n := range confiables {
		if n.Contains(p) {
			return true
		}
	}
	return false
}
