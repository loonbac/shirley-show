package store

import (
	"database/sql"
	"errors"
	"strings"
	"time"
)

// InteresadoData son los datos (ya validados) para crear o actualizar un interesado.
type InteresadoData struct {
	EventoID int64
	Nombre   string
	Contacto *string
	Personas int
}

const colsInteresado = `id, evento_id, nombre, contacto, personas, estado, fecha_creacion`

func escanearInteresado(row interface{ Scan(...any) error }) (*Interesado, error) {
	var it Interesado
	var contacto sql.NullString
	if err := row.Scan(&it.ID, &it.EventoID, &it.Nombre, &contacto, &it.Personas, &it.Estado, &it.FechaCreacion); err != nil {
		return nil, err
	}
	if contacto.Valid {
		it.Contacto = &contacto.String
	}
	return &it, nil
}

// ListarInteresados devuelve los interesados de un evento. Los que aún no confirman
// van primero (es donde la artista tiene trabajo por hacer).
func (s *Store) ListarInteresados(eventoID int64) ([]Interesado, error) {
	rows, err := s.db.Query("SELECT "+colsInteresado+
		" FROM interesados WHERE evento_id = ? ORDER BY estado DESC, fecha_creacion DESC, id DESC", eventoID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	lista := []Interesado{}
	for rows.Next() {
		it, err := escanearInteresado(rows)
		if err != nil {
			return nil, err
		}
		lista = append(lista, *it)
	}
	return lista, rows.Err()
}

func (s *Store) ObtenerInteresado(id int64) (*Interesado, error) {
	it, err := escanearInteresado(s.db.QueryRow("SELECT "+colsInteresado+" FROM interesados WHERE id = ?", id))
	if errors.Is(err, sql.ErrNoRows) {
		return nil, ErrNoEncontrado
	}
	return it, err
}

func (s *Store) CrearInteresado(d InteresadoData) (*Interesado, error) {
	ahora := time.Now().Format(time.RFC3339)
	res, err := s.db.Exec(
		"INSERT INTO interesados (evento_id, nombre, contacto, personas, estado, fecha_creacion) VALUES (?,?,?,?, 'Interesado', ?)",
		d.EventoID, strings.TrimSpace(d.Nombre), contactoSQL(d.Contacto), d.Personas, ahora)
	if err != nil {
		return nil, err
	}
	id, _ := res.LastInsertId()
	return s.ObtenerInteresado(id)
}

func (s *Store) ActualizarInteresado(id int64, d InteresadoData) (*Interesado, error) {
	if _, err := s.ObtenerInteresado(id); err != nil {
		return nil, err
	}
	_, err := s.db.Exec("UPDATE interesados SET nombre = ?, contacto = ?, personas = ? WHERE id = ?",
		strings.TrimSpace(d.Nombre), contactoSQL(d.Contacto), d.Personas, id)
	if err != nil {
		return nil, err
	}
	return s.ObtenerInteresado(id)
}

// MarcarInteresadoConfirmado lo pasa a 'Confirmado' (ya se le cobró y tiene su entrada).
func (s *Store) MarcarInteresadoConfirmado(id int64) (*Interesado, error) {
	if _, err := s.ObtenerInteresado(id); err != nil {
		return nil, err
	}
	if _, err := s.db.Exec("UPDATE interesados SET estado = 'Confirmado' WHERE id = ?", id); err != nil {
		return nil, err
	}
	return s.ObtenerInteresado(id)
}

func (s *Store) EliminarInteresado(id int64) error {
	res, err := s.db.Exec("DELETE FROM interesados WHERE id = ?", id)
	if err != nil {
		return err
	}
	if n, _ := res.RowsAffected(); n == 0 {
		return ErrNoEncontrado
	}
	return nil
}
