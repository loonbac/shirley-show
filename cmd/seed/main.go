// Seed: crea la administradora y un evento de ejemplo con entradas. Ejecutar una vez.
package main

import (
	"log"
	"time"

	"shirleyshow/internal/auth"
	"shirleyshow/internal/config"
	"shirleyshow/internal/store"
)

func main() {
	cfg := config.Cargar()
	st, err := store.Abrir(cfg.RutaDB)
	if err != nil {
		log.Fatalf("abrir base: %v", err)
	}
	defer st.Cerrar()

	// Administradora
	if _, err := st.ObtenerUsuario(cfg.AdminUsuario); err != nil {
		hash, _ := auth.Hash(cfg.AdminPassword)
		if err := st.CrearUsuario(cfg.AdminUsuario, hash); err != nil {
			log.Fatalf("crear admin: %v", err)
		}
		log.Printf("✓ Administradora creada: %s / %s", cfg.AdminUsuario, cfg.AdminPassword)
	}

	// Evento demo (solo si no hay eventos)
	if n, _ := st.ContarEventos(); n > 0 {
		log.Println("• Ya existen eventos; no se siembra el demo.")
		return
	}
	aforo := 120
	ev, err := st.CrearEvento(store.EventoData{
		Nombre:    "Noche de Boleros",
		FechaHora: time.Now().Add(30 * 24 * time.Hour).Format(time.RFC3339),
		Lugar:     "Teatro Municipal de Lima",
		Aforo:     &aforo,
		Tipos:     []store.TipoEntrada{{Nombre: "General", Precio: 30}, {Nombre: "VIP", Precio: 60}},
	})
	if err != nil {
		log.Fatalf("crear evento: %v", err)
	}

	ahora := time.Now().Format(time.RFC3339)
	demo := []struct {
		nombre, contacto, tipo string
		precio                 float64
		medio, pago, checkin   string
	}{
		{"María Quispe", "999111222", "VIP", 60, "Yape", "Pagado", "Pendiente"},
		{"José Ramírez", "988333444", "General", 30, "Efectivo", "Pagado", "Usada"},
		{"Lucía Torres", "977555666", "General", 30, "Yape", "Pendiente", "Pendiente"},
		{"Carlos Mendoza", "966777888", "VIP", 60, "Otra billetera", "Pagado", "Pendiente"},
		{"Ana Flores", "955999000", "General", 30, "Efectivo", "Pendiente", "Pendiente"},
	}
	ins := `INSERT INTO entradas
		(evento_id, nombre_comprador, contacto, tipo_entrada, precio, medio_pago, estado_pago, codigo_qr, estado_checkin, hora_ingreso, fecha_registro)
		VALUES (?,?,?,?,?,?,?,?,?,?,?)`
	for _, d := range demo {
		var hora any
		if d.checkin == "Usada" {
			hora = ahora
		}
		if _, err := st.DB().Exec(ins, ev.ID, d.nombre, d.contacto, d.tipo, d.precio, d.medio, d.pago,
			store.GenerarCodigoQR(), d.checkin, hora, ahora); err != nil {
			log.Fatalf("crear entrada demo: %v", err)
		}
	}
	log.Printf("✓ Evento demo \"%s\" con %d entradas creado.", ev.Nombre, len(demo))
}
