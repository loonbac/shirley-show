// Servidor de Shirley Show: API + frontend compilado, en un solo binario.
package main

import (
	"errors"
	"log"
	"net"
	"net/http"

	"shirleyshow/internal/api"
	"shirleyshow/internal/auth"
	"shirleyshow/internal/config"
	"shirleyshow/internal/store"
	"shirleyshow/internal/whatsapp"
)

func main() {
	cfg := config.Cargar()

	if cfg.SecretoGenerado {
		log.Println("ADVERTENCIA: JWT_SECRET no definido o muy corto. Se generó uno aleatorio temporal:")
		log.Println("  -> las sesiones se cerrarán al reiniciar. Define un JWT_SECRET de 32+ caracteres en producción.")
	}

	st, err := store.Abrir(cfg.RutaDB)
	if err != nil {
		log.Fatalf("no se pudo abrir la base de datos: %v", err)
	}
	defer st.Cerrar()

	// Asegura que exista la administradora (idempotente): así en producción
	// basta desplegar y entrar con las credenciales del entorno, sin seed manual.
	if err := asegurarAdmin(st, cfg); err != nil {
		log.Printf("aviso: no se pudo asegurar la administradora: %v", err)
	}

	var wa *whatsapp.Cliente
	if cfg.WhatsAppOn {
		wa, err = whatsapp.Nuevo(cfg.RutaStoreWA)
		if err != nil {
			log.Printf("WhatsApp no se pudo iniciar (continúo sin él): %v", err)
			wa = nil
		} else {
			log.Println("WhatsApp activado (whatsmeow)")
		}
	}

	srv := api.New(st, cfg, wa)
	addr := net.JoinHostPort(cfg.BindAddr, cfg.Puerto)
	log.Printf("Servidor escuchando en %s", addr)
	if err := http.ListenAndServe(addr, srv.Handler()); err != nil {
		log.Fatal(err)
	}
}

func asegurarAdmin(st *store.Store, cfg config.Config) error {
	if _, err := st.ObtenerUsuario(cfg.AdminUsuario); err == nil {
		return nil
	} else if !errors.Is(err, store.ErrNoEncontrado) {
		return err
	}
	hash, err := auth.Hash(cfg.AdminPassword)
	if err != nil {
		return err
	}
	log.Printf("Administradora creada: %s", cfg.AdminUsuario)
	return st.CrearUsuario(cfg.AdminUsuario, hash)
}
