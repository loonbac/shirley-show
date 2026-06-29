package store

import (
	"database/sql"
	"strings"
	"time"
)

// CheckinEntrada son los datos que se muestran al validar en puerta.
type CheckinEntrada struct {
	NombreComprador string  `json:"nombre_comprador"`
	TipoEntrada     string  `json:"tipo_entrada"`
	HoraIngreso     *string `json:"hora_ingreso,omitempty"`
}

// ResultadoCheckin: 'valida' | 'usada' | 'invalida'.
type ResultadoCheckin struct {
	Resultado string          `json:"resultado"`
	Entrada   *CheckinEntrada `json:"entrada,omitempty"`
}

// Checkin valida un QR escaneado en la puerta y marca la entrada como usada si corresponde.
func (s *Store) Checkin(eventoID int64, codigo string) (*ResultadoCheckin, error) {
	var id, evID int64
	var nombre, tipo, estadoCheckin string
	var horaIngreso sql.NullString
	err := s.db.QueryRow(
		"SELECT id, evento_id, nombre_comprador, tipo_entrada, estado_checkin, hora_ingreso FROM entradas WHERE codigo_qr = ?",
		strings.TrimSpace(codigo),
	).Scan(&id, &evID, &nombre, &tipo, &estadoCheckin, &horaIngreso)

	// No existe o pertenece a otro evento -> inválida para esta puerta.
	if err == sql.ErrNoRows || (err == nil && evID != eventoID) {
		return &ResultadoCheckin{Resultado: "invalida"}, nil
	}
	if err != nil {
		return nil, err
	}

	if estadoCheckin == "Usada" {
		var hora *string
		if horaIngreso.Valid {
			hora = &horaIngreso.String
		}
		return &ResultadoCheckin{Resultado: "usada", Entrada: &CheckinEntrada{nombre, tipo, hora}}, nil
	}

	ahora := time.Now().Format(time.RFC3339)
	if _, err := s.db.Exec("UPDATE entradas SET estado_checkin = 'Usada', hora_ingreso = ? WHERE id = ?", ahora, id); err != nil {
		return nil, err
	}
	return &ResultadoCheckin{Resultado: "valida", Entrada: &CheckinEntrada{nombre, tipo, &ahora}}, nil
}
