package config

import (
	"bufio"
	"os"
	"strconv"
	"strings"
)

// Config centraliza la configuración del servidor, leída de variables de entorno.
type Config struct {
	Puerto        string
	JWTSecret     string
	RutaDB        string
	AdminUsuario  string
	AdminPassword string
	WebDir        string // carpeta del frontend compilado (dist)
	WhatsAppOn    bool   // activa la integración con WhatsApp (whatsmeow)
	RutaStoreWA   string // sesión de WhatsApp (SQLite)
}

// Cargar lee un archivo .env opcional y las variables de entorno.
func Cargar() Config {
	cargarDotEnv(".env")
	return Config{
		Puerto:        env("PORT", "3001"),
		JWTSecret:     env("JWT_SECRET", "secreto-local-de-desarrollo"),
		RutaDB:        env("DB_PATH", "data/eventos.db"),
		AdminUsuario:  env("ADMIN_USUARIO", "shirleyshow"),
		AdminPassword: env("ADMIN_PASSWORD", "manuelitope123"),
		WebDir:        env("WEB_DIR", "dist"),
		WhatsAppOn:    env("WHATSAPP_ON", "false") == "true",
		RutaStoreWA:   env("WA_STORE_PATH", "data/whatsapp.db"),
	}
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
