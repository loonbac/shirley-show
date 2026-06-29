package store

import (
	"database/sql"
	"errors"
)

func (s *Store) ObtenerUsuario(usuario string) (*Usuario, error) {
	var u Usuario
	err := s.db.QueryRow("SELECT id, usuario, password FROM usuarios WHERE usuario = ?", usuario).
		Scan(&u.ID, &u.Usuario, &u.Password)
	if errors.Is(err, sql.ErrNoRows) {
		return nil, ErrNoEncontrado
	}
	return &u, err
}

func (s *Store) CrearUsuario(usuario, passwordHash string) error {
	_, err := s.db.Exec("INSERT INTO usuarios (usuario, password) VALUES (?, ?)", usuario, passwordHash)
	return err
}

func (s *Store) ContarEventos() (int, error) {
	var n int
	err := s.db.QueryRow("SELECT COUNT(*) FROM eventos").Scan(&n)
	return n, err
}
