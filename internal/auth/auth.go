// Package auth maneja contraseñas (bcrypt) y tokens de sesión (JWT).
package auth

import (
	"errors"
	"strconv"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"golang.org/x/crypto/bcrypt"
)

const duracion = 30 * 24 * time.Hour // sesión larga: uso interno

func Hash(password string) (string, error) {
	b, err := bcrypt.GenerateFromPassword([]byte(password), 10)
	return string(b), err
}

func Verificar(hash, password string) bool {
	return bcrypt.CompareHashAndPassword([]byte(hash), []byte(password)) == nil
}

// Claims del token: identifica a la administradora.
type Claims struct {
	Usuario string `json:"usuario"`
	jwt.RegisteredClaims
}

func FirmarToken(secret string, id int64, usuario string) (string, error) {
	claims := Claims{
		Usuario: usuario,
		RegisteredClaims: jwt.RegisteredClaims{
			Subject:   strconv.FormatInt(id, 10),
			ExpiresAt: jwt.NewNumericDate(time.Now().Add(duracion)),
		},
	}
	return jwt.NewWithClaims(jwt.SigningMethodHS256, claims).SignedString([]byte(secret))
}

func VerificarToken(secret, tokenStr string) (*Claims, error) {
	claims := &Claims{}
	_, err := jwt.ParseWithClaims(tokenStr, claims, func(t *jwt.Token) (any, error) {
		if _, ok := t.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, errors.New("método de firma inválido")
		}
		return []byte(secret), nil
	})
	if err != nil {
		return nil, err
	}
	return claims, nil
}
