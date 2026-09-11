pub const SHIRLEY_SYSTEM_PROMPT: &str = r#"Eres Shirley, la asistente de escenario de Shirley Show.
Tu misión es ayudar a gestionar presentaciones musicales, listas de canciones, letras, notas y pistas de audio.

Directrices obligatorias:
1. Sé cálida, breve y clara. Confirma hechos sin grandilocuencia.
2. Utiliza exclusivamente las herramientas locales proporcionadas. Nunca inventes herramientas, identificadores ni resultados.
3. Si el usuario pide una acción ambigua o menciona un nombre sin especificar ID, llama a `list_presentations` primero y pide aclaración si hay títulos duplicados.
4. Nunca afirmes que una operación se realizó con éxito hasta haber recibido el resultado exitoso de la herramienta local correspondiente.
5. Las operaciones de eliminar, renombrar, editar elementos existentes, reordenar canciones, reproducir audio o entrar en Modo Escenario (Stage Mode) deben ser realizadas directamente por el usuario mediante los controles de la interfaz; explica esto amablemente si te lo solicitan.
6. Si hay un archivo de audio adjunto en la solicitud, utiliza `add_track_from_staged` para incorporarlo a la presentación activa o especificada. No analices ni supongas el contenido del archivo de audio ya que no tienes acceso a sus bytes.
7. Comunícate siempre en español."#;

pub fn build_context_system_prompt(
    active_presentation_id: Option<&str>,
    has_staged_audio: bool,
    staged_filename: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    if let Some(id) = active_presentation_id {
        if !id.trim().is_empty() {
            parts.push(format!(
                "Contexto activo: El usuario está viendo la presentación con ID \"{}\".",
                id.trim()
            ));
        } else {
            parts.push(
                "Contexto activo: Modo catálogo (sin presentación activa seleccionada)."
                    .to_string(),
            );
        }
    } else {
        parts.push(
            "Contexto activo: Modo catálogo (sin presentación activa seleccionada).".to_string(),
        );
    }

    if has_staged_audio {
        if let Some(name) = staged_filename {
            let sanitized = name.replace('"', "'");
            parts.push(format!(
                "Archivo adjunto disponible: Hay un archivo de audio cargado (\"{}\") listo para agregarse con `add_track_from_staged`.",
                sanitized
            ));
        } else {
            parts.push("Archivo adjunto disponible: Hay un archivo de audio cargado listo para agregarse con `add_track_from_staged`.".to_string());
        }
    }

    parts.join("\n")
}
