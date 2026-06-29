package config

import (
	"bufio"
	"crypto/rand"
	"encoding/hex"
	"os"
	"strconv"
	"strings"
)

// Config centraliza la configuración del servidor, leída de variables de entorno.
type Config struct {
	Puerto        string
	BindAddr      string // interfaz donde escuchar ("" = todas; "127.0.0.1" = solo local/túnel)
	JWTSecret     string
	RutaDB        string
	AdminUsuario  string
	AdminPassword string
	WebDir          string // carpeta del frontend compilado (dist)
	WhatsAppOn      bool   // activa la integración con WhatsApp (whatsmeow)
	RutaStoreWA     string // sesión de WhatsApp (SQLite)
	SecretoGenerado bool   // true si el JWT_SECRET se generó al vuelo (no definido o muy corto)
}

// Cargar lee un archivo .env opcional y las variables de entorno.
func Cargar() Config {
	cargarDotEnv(".env")
	secreto, generado := resolverSecreto()
	return Config{
		Puerto:        env("PORT", "3001"),
		BindAddr:      env("BIND_ADDR", ""),
		JWTSecret:     secreto,
		RutaDB:        env("DB_PATH", "data/eventos.db"),
		AdminUsuario:  env("ADMIN_USUARIO", "shirleyshow"),
		AdminPassword: env("ADMIN_PASSWORD", "manuelitope123"),
		WebDir:        env("WEB_DIR", "dist"),
		WhatsAppOn:      env("WHATSAPP_ON", "false") == "true",
		RutaStoreWA:     env("WA_STORE_PATH", "data/whatsapp.db"),
		SecretoGenerado: generado,
	}
}

// resolverSecreto usa JWT_SECRET si tiene suficiente entropía (>=32 caracteres).
// Si no está definido o es muy corto/débil, genera uno aleatorio de 256 bits.
// Así nunca se firma con un secreto conocido del repositorio (evita falsificar tokens).
func resolverSecreto() (string, bool) {
	s := os.Getenv("JWT_SECRET")
	if len(s) >= 32 {
		return s, false
	}
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		return s, false // improbable; en el peor caso usa lo que haya
	}
	return hex.EncodeToString(b), true
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
