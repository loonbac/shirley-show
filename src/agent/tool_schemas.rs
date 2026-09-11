use serde_json::json;

use crate::domain::agent_tool::ToolDefinition;

pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "create_presentation".to_string(),
            description: "Crea una nueva presentación o repertorio de canciones con el nombre indicado.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Nombre de la presentación o show a crear."
                    }
                },
                "required": ["name"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "list_presentations".to_string(),
            description: "Lista todas las presentaciones disponibles con sus estadísticas y conteos de elementos.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "get_presentation".to_string(),
            description: "Obtiene los detalles completos de una presentación, incluyendo la lista ordenada de elementos (letras, notas, pistas).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentation_id": {
                        "type": "string",
                        "description": "Identificador único de la presentación a consultar."
                    }
                },
                "required": ["presentation_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "add_lyrics".to_string(),
            description: "Añade una letra de canción al final de la secuencia de una presentación existente.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentation_id": {
                        "type": "string",
                        "description": "Identificador único de la presentación."
                    },
                    "text": {
                        "type": "string",
                        "description": "Contenido de la letra de la canción (admite saltos de línea y acordes)."
                    }
                },
                "required": ["presentation_id", "text"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "add_note".to_string(),
            description: "Añade una nota o anotación de escenario al final de la secuencia de una presentación existente.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentation_id": {
                        "type": "string",
                        "description": "Identificador único de la presentación."
                    },
                    "text": {
                        "type": "string",
                        "description": "Texto de la nota (indicaciones técnicas, afinaciones, recordatorios)."
                    }
                },
                "required": ["presentation_id", "text"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "add_track_from_staged".to_string(),
            description: "Incorpora el archivo de audio adjunto o preparado en la solicitud actual como una pista en la presentación especificada.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentation_id": {
                        "type": "string",
                        "description": "Identificador único de la presentación donde se agregará la pista."
                    },
                    "title": {
                        "type": "string",
                        "description": "Título de la pista de audio."
                    },
                    "staged_audio_id": {
                        "type": "string",
                        "description": "Identificador opcional del audio preparado previamente para esta solicitud."
                    }
                },
                "required": ["presentation_id", "title"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "download_youtube_track".to_string(),
            description: "Descarga el audio de un video de YouTube y lo prepara temporalmente como archivo staged para agregarlo como pista.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL pública del video de YouTube a descargar (youtube.com o youtu.be)."
                    },
                    "title": {
                        "type": "string",
                        "description": "Título opcional personalizado para la pista de audio."
                    }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        },
    ]
}
