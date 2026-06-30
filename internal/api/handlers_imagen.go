package api

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"shirleyshow/internal/store"
)

// Tamaño máximo del afiche subido (10 MB) y tipos permitidos.
const maxImagenBytes = 10 << 20

var extPorTipo = map[string]string{
	"image/jpeg": ".jpg",
	"image/png":  ".png",
	"image/webp": ".webp",
}

// subirImagen recibe el afiche (multipart, campo "imagen"), lo guarda en MEDIA_DIR
// y actualiza el evento. Reemplaza la imagen anterior si la había.
func (s *Server) subirImagen(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	ev, err := s.st.ObtenerEvento(id) // 404 si no existe; también nos da la imagen anterior
	if err != nil {
		return err
	}

	r.Body = http.MaxBytesReader(w, r.Body, maxImagenBytes+1024)
	archivo, cab, err := r.FormFile("imagen")
	if err != nil {
		return errDatos("Falta el archivo de imagen (campo \"imagen\")")
	}
	defer archivo.Close()
	if cab.Size > maxImagenBytes {
		return errDatos("La imagen supera el tamaño máximo de 10 MB")
	}

	// Detecta el tipo por el contenido real (no confía en la extensión del cliente).
	cabecera := make([]byte, 512)
	n, _ := io.ReadFull(archivo, cabecera)
	tipo := http.DetectContentType(cabecera[:n])
	ext, ok := extPorTipo[tipo]
	if !ok {
		return errDatos("Formato no permitido. Usa JPG, PNG o WebP")
	}
	if _, err := archivo.Seek(0, io.SeekStart); err != nil {
		return err
	}

	if err := os.MkdirAll(s.cfg.MediaDir, 0o755); err != nil {
		return err
	}
	nombre := fmt.Sprintf("%d-%s%s", id, aleatorioHex(8), ext)
	destino := filepath.Join(s.cfg.MediaDir, nombre)
	f, err := os.Create(destino)
	if err != nil {
		return err
	}
	if _, err := io.Copy(f, archivo); err != nil {
		f.Close()
		os.Remove(destino)
		return err
	}
	if err := f.Close(); err != nil {
		os.Remove(destino)
		return err
	}

	if err := s.st.ActualizarImagen(id, nombre); err != nil {
		os.Remove(destino)
		return err
	}
	s.borrarMedia(ev.Imagen) // limpia el afiche anterior (best-effort)

	actualizado, err := s.st.ObtenerEvento(id)
	if err != nil {
		return err
	}
	escribirJSON(w, http.StatusOK, actualizado)
	return nil
}

// quitarImagen borra el afiche del evento.
func (s *Server) quitarImagen(w http.ResponseWriter, r *http.Request) error {
	id, err := idDeRuta(r, "id")
	if err != nil {
		return err
	}
	ev, err := s.st.ObtenerEvento(id)
	if err != nil {
		return err
	}
	if err := s.st.ActualizarImagen(id, ""); err != nil {
		return err
	}
	s.borrarMedia(ev.Imagen)
	escribirJSON(w, http.StatusOK, map[string]bool{"eliminada": true})
	return nil
}

// servirMedia entrega un archivo de MEDIA_DIR. Pública (la página vive tras el túnel
// privado); filepath.Base evita salir del directorio (path traversal).
func (s *Server) servirMedia(w http.ResponseWriter, r *http.Request) {
	nombre := filepath.Base(r.PathValue("archivo"))
	if nombre == "." || nombre == "/" || strings.TrimSpace(nombre) == "" {
		http.NotFound(w, r)
		return
	}
	w.Header().Set("Cache-Control", "public, max-age=31536000, immutable")
	http.ServeFile(w, r, filepath.Join(s.cfg.MediaDir, nombre))
}

// borrarMedia elimina un archivo de MEDIA_DIR si el nombre no es nil/vacío.
func (s *Server) borrarMedia(nombre *string) {
	if nombre == nil || *nombre == "" {
		return
	}
	_ = os.Remove(filepath.Join(s.cfg.MediaDir, filepath.Base(*nombre)))
}

func aleatorioHex(n int) string {
	b := make([]byte, n)
	if _, err := rand.Read(b); err != nil {
		return "img"
	}
	return hex.EncodeToString(b)
}

// borrarImagenDeEvento limpia el archivo de afiche al eliminar un evento.
func (s *Server) borrarImagenDeEvento(ev *store.Evento) {
	if ev != nil {
		s.borrarMedia(ev.Imagen)
	}
}
