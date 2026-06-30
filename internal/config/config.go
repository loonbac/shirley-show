package config

import (
	"bufio"
	"crypto/rand"
	"encoding/hex"
	"net"
	"os"
	"strconv"
	"strings"
)

// Valores por defecto del repositorio que NUNCA deben usarse en producción.
const (
	passwordPorDefecto = "manuelitope123"
	// Rangos de confianza para creer en cabeceras de IP (CF-Connecting-IP / XFF):
	// loopback + redes privadas (la app solo es alcanzable vía el túnel/red local).
	cidrsPorDefecto = "127.0.0.0/8,::1/128,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,fc00::/7"
)

// secretosInseguros son JWT_SECRET conocidos (placeholder del repo) que se rechazan
// aunque cumplan la longitud mínima, para no firmar con un secreto público.
var secretosInseguros = map[string]bool{
	"cambia-esto-por-un-secreto-largo-y-aleatorio": true,
}

// Config centraliza la configuración del servidor, leída de variables de entorno.
type Config struct {
	Puerto          string
	BindAddr        string // interfaz donde escuchar ("" = todas; "127.0.0.1" = solo local/túnel)
	JWTSecret       string
	RutaDB          string
	MediaDir        string // carpeta donde se guardan las imágenes subidas (afiches)
	AdminUsuario    string
	AdminPassword   string
	WebDir          string // carpeta del frontend compilado (dist)
	WhatsAppOn      bool   // activa la integración con WhatsApp (whatsmeow)
	RutaStoreWA     string // sesión de WhatsApp (SQLite)
	SecretoGenerado bool   // true si el JWT_SECRET se generó al vuelo (no definido, corto o placeholder)

	TrustedProxies     []*net.IPNet // redes cuyo CF-Connecting-IP / X-Forwarded-For se respeta
	CORSOrigin         string       // origen permitido para CORS ("" = solo mismo origen)
	PublicBaseURL      string       // base pública para enlaces (evita Host header injection)
	PasswordPorDefecto bool         // true si ADMIN_PASSWORD es el valor por defecto del repo
}

// Cargar lee un archivo .env opcional y las variables de entorno.
func Cargar() Config {
	cargarDotEnv(".env")
	secreto, generado := resolverSecreto()
	adminPass := env("ADMIN_PASSWORD", passwordPorDefecto)
	return Config{
		Puerto:          env("PORT", "3001"),
		BindAddr:        env("BIND_ADDR", ""),
		JWTSecret:       secreto,
		RutaDB:          env("DB_PATH", "data/eventos.db"),
		MediaDir:        env("MEDIA_DIR", "data/media"),
		AdminUsuario:    env("ADMIN_USUARIO", "shirleyshow"),
		AdminPassword:   adminPass,
		WebDir:          env("WEB_DIR", "dist"),
		WhatsAppOn:      env("WHATSAPP_ON", "false") == "true",
		RutaStoreWA:     env("WA_STORE_PATH", "data/whatsapp.db"),
		SecretoGenerado: generado,

		TrustedProxies:     parsearCIDRs(env("TRUSTED_PROXY_CIDRS", cidrsPorDefecto)),
		CORSOrigin:         env("CORS_ORIGIN", ""),
		PublicBaseURL:      env("PUBLIC_BASE_URL", ""),
		PasswordPorDefecto: adminPass == passwordPorDefecto,
	}
}

// resolverSecreto usa JWT_SECRET si tiene suficiente entropía (>=32 caracteres) y NO
// es un valor por defecto conocido. Si falta, es corto o es el placeholder del repo,
// genera uno aleatorio de 256 bits. Así nunca se firma con un secreto público (lo que
// permitiría falsificar tokens de administradora sin contraseña).
func resolverSecreto() (string, bool) {
	s := os.Getenv("JWT_SECRET")
	if len(s) >= 32 && !esSecretoInseguro(s) {
		return s, false
	}
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return s, false // improbable; en el peor caso usa lo que haya
	}
	return hex.EncodeToString(b), true
}

// esSecretoInseguro rechaza el placeholder del repositorio y cualquier "cambia-esto…".
func esSecretoInseguro(s string) bool {
	return secretosInseguros[s] || strings.HasPrefix(strings.ToLower(s), "cambia-esto")
}

// parsearCIDRs convierte una lista separada por comas en redes; ignora entradas inválidas.
func parsearCIDRs(lista string) []*net.IPNet {
	var nets []*net.IPNet
	for _, parte := range strings.Split(lista, ",") {
		if parte = strings.TrimSpace(parte); parte == "" {
			continue
		}
		if _, n, err := net.ParseCIDR(parte); err == nil {
			nets = append(nets, n)
		}
	}
	return nets
}

func env(clave, porDefecto string) string {
	if v, ok := os.LookupEnv(clave); ok && v != "" {
		return v
	}
	return porDefecto
}

// PuertoNum devuelve el puerto como número (con respaldo a 3001).
func (c Config) PuertoNum() int {
	n, err := strconv.Atoi(c.Puerto)
	if err != nil {
		return 3001
	}
	return n
}

// cargarDotEnv carga variables desde un archivo .env si existe (no sobreescribe las ya definidas).
func cargarDotEnv(ruta string) {
	f, err := os.Open(ruta)
	if err != nil {
		return
	}
	defer f.Close()
	sc := bufio.NewScanner(f)
	for sc.Scan() {
		linea := strings.TrimSpace(sc.Text())
		if linea == "" || strings.HasPrefix(linea, "#") {
			continue
		}
		clave, valor, ok := strings.Cut(linea, "=")
		if !ok {
			continue
		}
		clave, valor = strings.TrimSpace(clave), strings.TrimSpace(valor)
		if _, existe := os.LookupEnv(clave); !existe {
			os.Setenv(clave, valor)
		}
	}
}
