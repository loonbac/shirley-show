use serde_json::json;

use crate::domain::agent_tool::ToolDefinition;

pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // Presentation tools (8)
        ToolDefinition {
            name: "create_presentation".to_string(),
            description: "Crea presentación.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "list_presentations".to_string(),
            description: "Lista presentaciones.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "get_presentation".to_string(),
            description: "Detalle de presentación.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentation_id": { "type": "string" }
                },
                "required": ["presentation_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "add_lyrics".to_string(),
            description: "Añade letra.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentation_id": { "type": "string" },
                    "text": { "type": "string" }
                },
                "required": ["presentation_id", "text"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "add_note".to_string(),
            description: "Añade nota.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentation_id": { "type": "string" },
                    "text": { "type": "string" }
                },
                "required": ["presentation_id", "text"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "add_track_from_staged".to_string(),
            description: "Agrega pista staging.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentation_id": { "type": "string" },
                    "title": { "type": "string" },
                    "staged_audio_id": { "type": "string" }
                },
                "required": ["presentation_id", "title"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "download_youtube_track".to_string(),
            description: "Descarga YouTube.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string" },
                    "title": { "type": "string" }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "separate_vocals".to_string(),
            description: "Procesa una pista con los modelos vocales. mode=chorus elimina la voz principal y conserva coros más instrumentación; mode=instrumental elimina todas las voces. El resultado queda temporal y después debe agregarse con add_track_from_staged.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["chorus", "instrumental"]
                    },
                    "staged_audio_id": { "type": "string" },
                    "track_id": { "type": "string" },
                    "youtube_url": { "type": "string" }
                },
                "required": ["mode"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "renombrar_presentacion".to_string(),
            description: "Renombra presentación.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentacion_id": { "type": "string" },
                    "nuevo_nombre": { "type": "string" }
                },
                "required": ["presentacion_id", "nuevo_nombre"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "reordenar_elemento".to_string(),
            description: "Reordena elemento.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentacion_id": { "type": "string" },
                    "elemento_id": { "type": "string" },
                    "posicion_destino": {
                        "type": "integer",
                        "minimum": 1
                    },
                    "direccion": {
                        "type": "string",
                        "enum": ["arriba", "abajo"]
                    }
                },
                "required": ["presentacion_id", "elemento_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "editar_elemento".to_string(),
            description: "Edita elemento.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentacion_id": { "type": "string" },
                    "elemento_id": { "type": "string" },
                    "texto": { "type": "string" },
                    "titulo": { "type": "string" }
                },
                "required": ["presentacion_id", "elemento_id"],
                "additionalProperties": false
            }),
        },
        // Ticketing tools (8 + 5 safe = 13)
        ToolDefinition {
            name: "crear_evento".to_string(),
            description: "Crea evento.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "nombre": { "type": "string" },
                    "fecha_hora": { "type": "string" },
                    "lugar": { "type": "string" },
                    "tipos_de_entrada": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "nombre": { "type": "string" },
                                "precio": { "type": "number", "minimum": 0.0 },
                                "capacidad": { "type": "integer", "minimum": 1 }
                            },
                            "required": ["nombre", "precio", "capacidad"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["nombre", "fecha_hora", "lugar", "tipos_de_entrada"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "listar_eventos".to_string(),
            description: "Lista eventos.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "incluir_finalizados": { "type": "boolean" }
                },
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "vender_entradas".to_string(),
            description: "Vende entradas.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": { "type": "string" },
                    "nombre_comprador": { "type": "string" },
                    "contacto": { "type": "string" },
                    "tipo_entrada": { "type": "string" },
                    "cantidad": { "type": "integer", "minimum": 1 },
                    "medio_pago": {
                        "type": "string",
                        "enum": ["Yape", "Efectivo", "Cortesía"]
                    },
                    "estado_pago": {
                        "type": "string",
                        "enum": ["Pagado", "Pendiente"]
                    }
                },
                "required": [
                    "evento_id",
                    "nombre_comprador",
                    "contacto",
                    "tipo_entrada",
                    "cantidad",
                    "medio_pago",
                    "estado_pago"
                ],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "confirmar_pago_entrada".to_string(),
            description: "Confirma pago entrada.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "entrada_id": { "type": "string" }
                },
                "required": ["entrada_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "registrar_gasto".to_string(),
            description: "Registra gasto.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": { "type": "string" },
                    "categoria": {
                        "type": "string",
                        "enum": [
                            "Local/Alquiler",
                            "Sonido/Luces",
                            "Músicos/Personal",
                            "Publicidad/Marketing",
                            "Bebidas/Catering",
                            "Transporte",
                            "Otros"
                        ]
                    },
                    "monto": { "type": "number", "minimum": 0.0 },
                    "descripcion": { "type": "string" },
                    "fecha": { "type": "string" }
                },
                "required": ["evento_id", "categoria", "monto", "descripcion"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "resumen_evento".to_string(),
            description: "Resumen evento.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": { "type": "string" }
                },
                "required": ["evento_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "registrar_interesado".to_string(),
            description: "Registra interesado.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": { "type": "string" },
                    "nombre": { "type": "string" },
                    "contacto": { "type": "string" },
                    "canal": { "type": "string" },
                    "personas": { "type": "integer", "minimum": 1 },
                    "promotora": { "type": "string" }
                },
                "required": ["evento_id", "nombre", "contacto", "canal", "personas", "promotora"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "checkin_entrada".to_string(),
            description: "Valida QR entrada.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": { "type": "string" },
                    "codigo": { "type": "string" }
                },
                "required": ["evento_id", "codigo"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "editar_evento".to_string(),
            description: "Edita evento.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": { "type": "string" },
                    "nombre": { "type": "string" },
                    "fecha_hora": { "type": "string" },
                    "lugar": { "type": "string" },
                    "tipos_de_entrada": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "nombre": { "type": "string" },
                                "precio": { "type": "number", "minimum": 0.0 },
                                "capacidad": { "type": "integer", "minimum": 1 }
                            },
                            "required": ["nombre", "precio", "capacidad"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["evento_id", "nombre", "fecha_hora", "lugar", "tipos_de_entrada"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "reabrir_evento".to_string(),
            description: "Reabre evento.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": { "type": "string" }
                },
                "required": ["evento_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "editar_gasto".to_string(),
            description: "Edita gasto.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "gasto_id": { "type": "string" },
                    "categoria": {
                        "type": "string",
                        "enum": [
                            "Local/Alquiler",
                            "Sonido/Luces",
                            "Músicos/Personal",
                            "Publicidad/Marketing",
                            "Bebidas/Catering",
                            "Transporte",
                            "Otros"
                        ]
                    },
                    "monto": { "type": "number", "minimum": 0.0 },
                    "descripcion": { "type": "string" },
                    "fecha": { "type": "string" }
                },
                "required": ["gasto_id", "categoria", "monto", "descripcion", "fecha"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "editar_interesado".to_string(),
            description: "Edita interesado.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "interesado_id": { "type": "string" },
                    "nombre": { "type": "string" },
                    "contacto": { "type": "string" },
                    "canal": { "type": "string" },
                    "personas": { "type": "integer", "minimum": 1 },
                    "promotora": { "type": "string" },
                    "estado": {
                        "type": "string",
                        "enum": ["Pendiente", "Confirmado", "Cancelado"]
                    }
                },
                "required": [
                    "interesado_id",
                    "nombre",
                    "contacto",
                    "canal",
                    "personas",
                    "promotora",
                    "estado"
                ],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "confirmar_interesado".to_string(),
            description: "Convierte interesado a venta.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "interesado_id": { "type": "string" },
                    "tipo_entrada": { "type": "string" },
                    "cantidad": { "type": "integer", "minimum": 1 },
                    "medio_pago": {
                        "type": "string",
                        "enum": ["Yape", "Efectivo", "Cortesía"]
                    },
                    "estado_pago": {
                        "type": "string",
                        "enum": ["Pagado", "Pendiente"]
                    }
                },
                "required": [
                    "interesado_id",
                    "tipo_entrada",
                    "cantidad",
                    "medio_pago",
                    "estado_pago"
                ],
                "additionalProperties": false
            }),
        },
        // Destructive presentation tools (Slice 2)
        ToolDefinition {
            name: "eliminar_presentacion".to_string(),
            description: "Elimina presentación.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentacion_id": { "type": "string" }
                },
                "required": ["presentacion_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "eliminar_elemento".to_string(),
            description: "Elimina elemento.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentacion_id": { "type": "string" },
                    "elemento_id": { "type": "string" }
                },
                "required": ["presentacion_id", "elemento_id"],
                "additionalProperties": false
            }),
        },
        // Destructive ticketing tools (Slice 2)
        ToolDefinition {
            name: "eliminar_evento".to_string(),
            description: "Elimina evento.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": { "type": "string" }
                },
                "required": ["evento_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "finalizar_evento".to_string(),
            description: "Finaliza evento.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": { "type": "string" }
                },
                "required": ["evento_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "eliminar_entrada".to_string(),
            description: "Elimina entrada.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "entrada_id": { "type": "string" }
                },
                "required": ["entrada_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "eliminar_gasto".to_string(),
            description: "Elimina gasto.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "gasto_id": { "type": "string" }
                },
                "required": ["gasto_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "eliminar_interesado".to_string(),
            description: "Elimina interesado.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "interesado_id": { "type": "string" }
                },
                "required": ["interesado_id"],
                "additionalProperties": false
            }),
        },
        // Poster upload tool (Slice 3)
        ToolDefinition {
            name: "subir_afiche".to_string(),
            description: "Asigna afiche.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": { "type": "string" }
                },
                "required": ["evento_id"],
                "additionalProperties": false
            }),
        },
    ]
}
