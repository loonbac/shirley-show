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

// ipCliente obtiene la IP real del cliente (respetando Cloudflare / proxies).
func ipCliente(r *http.Request) string {
	if cf := r.Header.Get("CF-Connecting-IP"); cf != "" {
		return cf
	}
	if xff := r.Header.Get("X-Forwarded-For"); xff != "" {
		return strings.TrimSpace(strings.Split(xff, ",")[0])
	}
	if host, _, err := net.SplitHostPort(r.RemoteAddr); err == nil {
		return host
	}
	return r.RemoteAddr
}
