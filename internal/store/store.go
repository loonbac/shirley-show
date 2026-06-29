// Package store es la capa de datos: abre SQLite y expone el acceso a los datos.
package store

import (
	"database/sql"
	_ "embed"
	"fmt"
	"os"
	"path/filepath"

	_ "modernc.org/sqlite" // driver SQLite puro en Go (sin CGO)
)

//go:embed schema.sql
var schema string

// Store envuelve la conexión a la base de datos.
type Store struct {
	db *sql.DB
}

// Abrir abre (o crea) la base SQLite, aplica el esquema y devuelve el Store.
func Abrir(ruta string) (*Store, error) {
	if err := os.MkdirAll(filepath.Dir(ruta), 0o755); err != nil {
		return nil, err
	}
	dsn := fmt.Sprintf("file:%s?_pragma=foreign_keys(1)&_pragma=journal_mode(WAL)&_pragma=busy_timeout(5000)", ruta)
	db, err := sql.Open("sqlite", dsn)
	if err != nil {
		return nil, err
	}
	// SQLite escribe mejor con una sola conexión de escritura: evita "database is locked".
	db.SetMaxOpenConns(1)
	if _, err := db.Exec(schema); err != nil {
		return nil, fmt.Errorf("aplicar esquema: %w", err)
	}
	return &Store{db: db}, nil
}

// DB expone la conexión cruda (la usa el seed y whatsmeow).
func (s *Store) DB() *sql.DB { return s.db }

func (s *Store) Cerrar() error { return s.db.Close() }
