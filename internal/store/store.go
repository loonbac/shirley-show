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
	migrar(db)
	return &Store{db: db}, nil
}

// migrar agrega columnas nuevas a bases ya existentes (idempotente: ignora si ya están).
func migrar(db *sql.DB) {
	// ALTER ... ADD COLUMN falla si la columna ya existe; ese error se ignora a propósito.
	_, _ = db.Exec("ALTER TABLE eventos ADD COLUMN finalizado INTEGER NOT NULL DEFAULT 0")
	_, _ = db.Exec("ALTER TABLE eventos ADD COLUMN imagen TEXT")
	_, _ = db.Exec("ALTER TABLE entradas ADD COLUMN scan_token TEXT")
	backfillScanTokens(db)
	_, _ = db.Exec("CREATE INDEX IF NOT EXISTS idx_entradas_scantoken ON entradas(scan_token)")
}

// backfillScanTokens asigna un token corto a las entradas que aún no lo tienen
// (filas creadas antes de existir la columna).
func backfillScanTokens(db *sql.DB) {
	rows, err := db.Query("SELECT id FROM entradas WHERE scan_token IS NULL OR scan_token = ''")
	if err != nil {
		return
	}
	var ids []int64
	for rows.Next() {
		var id int64
		if rows.Scan(&id) == nil {
			ids = append(ids, id)
		}
	}
	rows.Close()
	for _, id := range ids {
		_, _ = db.Exec("UPDATE entradas SET scan_token = ? WHERE id = ?", generarToken(), id)
	}
}

// DB expone la conexión cruda (la usa el seed y whatsmeow).
func (s *Store) DB() *sql.DB { return s.db }

func (s *Store) Cerrar() error { return s.db.Close() }
