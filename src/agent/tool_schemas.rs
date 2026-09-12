use serde_json::json;

use crate::domain::agent_tool::ToolDefinition;

pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // Presentation tools (8)
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
        ToolDefinition {
            name: "separate_vocals".to_string(),
            description: "Separa la voz principal de una pista de audio (modo 'chorus' para conservar coros de fondo o 'instrumental' para pista completamente instrumental) produciendo un audio preparado temporalmente.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["chorus", "instrumental"],
                        "description": "Modo de separación: 'chorus' (conserva coros de fondo) o 'instrumental' (pista puramente instrumental)."
                    },
                    "staged_audio_id": {
                        "type": "string",
                        "description": "Identificador opcional de audio preparado en staging a separar."
                    },
                    "track_id": {
                        "type": "string",
                        "description": "Identificador opcional de una pista existente en la presentación actual a separar."
                    },
                    "youtube_url": {
                        "type": "string",
                        "description": "URL pública de YouTube opcional a descargar y separar."
                    }
                },
                "required": ["mode"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "renombrar_presentacion".to_string(),
            description: "Renombra una presentación existente con un nuevo título.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentacion_id": {
                        "type": "string",
                        "description": "Identificador único de la presentación a renombrar."
                    },
                    "nuevo_nombre": {
                        "type": "string",
                        "description": "Nuevo título o nombre para la presentación."
                    }
                },
                "required": ["presentacion_id", "nuevo_nombre"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "reordenar_elemento".to_string(),
            description: "Cambia la posición de un elemento dentro de una presentación por posición destino absoluta (1-based) o dirección ('arriba' o 'abajo').".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentacion_id": {
                        "type": "string",
                        "description": "Identificador único de la presentación."
                    },
                    "elemento_id": {
                        "type": "string",
                        "description": "Identificador único del elemento a reordenar."
                    },
                    "posicion_destino": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Posición destino absoluta (1-based). Mutuamente excluyente con 'direccion'."
                    },
                    "direccion": {
                        "type": "string",
                        "enum": ["arriba", "abajo"],
                        "description": "Dirección de desplazamiento: 'arriba' o 'abajo'. Mutuamente excluyente con 'posicion_destino'."
                    }
                },
                "required": ["presentacion_id", "elemento_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "editar_elemento".to_string(),
            description: "Edita el contenido de texto de una letra/nota o el título de una pista de audio en una presentación.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentacion_id": {
                        "type": "string",
                        "description": "Identificador único de la presentación."
                    },
                    "elemento_id": {
                        "type": "string",
                        "description": "Identificador único del elemento a editar."
                    },
                    "texto": {
                        "type": "string",
                        "description": "Nuevo contenido de texto (para letras o notas). Mutuamente excluyente con 'titulo'."
                    },
                    "titulo": {
                        "type": "string",
                        "description": "Nuevo título (para pistas de audio). Mutuamente excluyente con 'texto'."
                    }
                },
                "required": ["presentacion_id", "elemento_id"],
                "additionalProperties": false
            }),
        },

        // Ticketing tools (8 + 5 safe = 13)
        ToolDefinition {
            name: "crear_evento".to_string(),
            description: "Crea un nuevo evento musical o show con fecha, lugar y tipos de entrada configurados.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "nombre": {
                        "type": "string",
                        "description": "Nombre o título del evento."
                    },
                    "fecha_hora": {
                        "type": "string",
                        "description": "Fecha y hora del evento en formato ISO 8601 (YYYY-MM-DDTHH:MM)."
                    },
                    "lugar": {
                        "type": "string",
                        "description": "Lugar, recinto o dirección del evento."
                    },
                    "tipos_de_entrada": {
                        "type": "array",
                        "description": "Lista de tipos de entrada para el evento.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "nombre": {
                                    "type": "string",
                                    "description": "Nombre del tipo o zona de entrada (ej: 'General', 'VIP', 'Cortesía')."
                                },
                                "precio": {
                                    "type": "number",
                                    "minimum": 0.0,
                                    "description": "Precio unitario en soles (S/)."
                                },
                                "capacidad": {
                                    "type": "integer",
                                    "minimum": 1,
                                    "description": "Capacidad o aforo disponible para este tipo."
                                }
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
            description: "Lista todos los eventos disponibles con sus detalles y estado.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "incluir_finalizados": {
                        "type": "boolean",
                        "description": "Si es verdadero, incluye eventos finalizados en la lista. Por defecto falso."
                    }
                },
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "vender_entradas".to_string(),
            description: "Vende o emite una o más entradas para un evento activo.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": {
                        "type": "string",
                        "description": "Identificador único del evento."
                    },
                    "nombre_comprador": {
                        "type": "string",
                        "description": "Nombre del comprador o titular de las entradas."
                    },
                    "contacto": {
                        "type": "string",
                        "description": "Teléfono o celular del comprador para envío de entrada."
                    },
                    "tipo_entrada": {
                        "type": "string",
                        "description": "Nombre exacto del tipo de entrada a vender."
                    },
                    "cantidad": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Cantidad de entradas a emitir."
                    },
                    "medio_pago": {
                        "type": "string",
                        "enum": ["Yape", "Efectivo", "Cortesía"],
                        "description": "Medio de pago: 'Yape', 'Efectivo' o 'Cortesía'."
                    },
                    "estado_pago": {
                        "type": "string",
                        "enum": ["Pagado", "Pendiente"],
                        "description": "Estado de la entrada: 'Pagado' (pagado o cortesía) o 'Pendiente' (reserva)."
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
            description: "Confirma el pago de una entrada previamente en estado 'Pendiente' cambiándola a 'Pagado'.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "entrada_id": {
                        "type": "string",
                        "description": "Identificador único de la entrada a confirmar."
                    }
                },
                "required": ["entrada_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "registrar_gasto".to_string(),
            description: "Registra un gasto o desembolso financiero asociado a un evento específico.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": {
                        "type": "string",
                        "description": "Identificador único del evento."
                    },
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
                        ],
                        "description": "Categoría contable del gasto."
                    },
                    "monto": {
                        "type": "number",
                        "minimum": 0.0,
                        "description": "Monto del gasto en soles (S/)."
                    },
                    "descripcion": {
                        "type": "string",
                        "description": "Descripción o detalle del gasto realizado."
                    },
                    "fecha": {
                        "type": "string",
                        "description": "Fecha del gasto en formato YYYY-MM-DD (opcional, por defecto fecha actual)."
                    }
                },
                "required": ["evento_id", "categoria", "monto", "descripcion"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "resumen_evento".to_string(),
            description: "Obtiene el resumen financiero y operativo consolidado de un evento (ingresos, cobranzas, pendientes, balance y gastos).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": {
                        "type": "string",
                        "description": "Identificador único del evento."
                    }
                },
                "required": ["evento_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "registrar_interesado".to_string(),
            description: "Registra una persona interesada o reserva potencial para un evento.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": {
                        "type": "string",
                        "description": "Identificador único del evento."
                    },
                    "nombre": {
                        "type": "string",
                        "description": "Nombre de la persona interesada."
                    },
                    "contacto": {
                        "type": "string",
                        "description": "Teléfono o celular de contacto."
                    },
                    "canal": {
                        "type": "string",
                        "description": "Canal por el cual se comunicó (ej: WhatsApp, Instagram, Personal)."
                    },
                    "personas": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Número de personas estimadas en el grupo."
                    },
                    "promotora": {
                        "type": "string",
                        "description": "Nombre de la promotora o persona responsable del contacto."
                    }
                },
                "required": ["evento_id", "nombre", "contacto", "canal", "personas", "promotora"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "checkin_entrada".to_string(),
            description: "Valida en puerta el código QR secreto de 32 caracteres hexadecimales de una entrada.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": {
                        "type": "string",
                        "description": "Identificador único del evento."
                    },
                    "codigo": {
                        "type": "string",
                        "description": "Código secreto QR de 32 caracteres hexadecimales."
                    }
                },
                "required": ["evento_id", "codigo"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "editar_evento".to_string(),
            description: "Actualiza los datos básicos de un evento y sus tipos de entrada.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": {
                        "type": "string",
                        "description": "Identificador único del evento a editar."
                    },
                    "nombre": {
                        "type": "string",
                        "description": "Nuevo nombre o título del evento."
                    },
                    "fecha_hora": {
                        "type": "string",
                        "description": "Fecha y hora del evento en formato ISO 8601 (YYYY-MM-DDTHH:MM)."
                    },
                    "lugar": {
                        "type": "string",
                        "description": "Lugar, recinto o dirección del evento."
                    },
                    "tipos_de_entrada": {
                        "type": "array",
                        "description": "Lista de tipos de entrada para el evento.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "nombre": {
                                    "type": "string",
                                    "description": "Nombre del tipo o zona de entrada (ej: 'General', 'VIP', 'Cortesía')."
                                },
                                "precio": {
                                    "type": "number",
                                    "minimum": 0.0,
                                    "description": "Precio unitario en soles (S/)."
                                },
                                "capacidad": {
                                    "type": "integer",
                                    "minimum": 1,
                                    "description": "Capacidad o aforo disponible para este tipo."
                                }
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
            description: "Reabre un evento previamente finalizado cambiando su estado a 'Activo'.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": {
                        "type": "string",
                        "description": "Identificador único del evento a reabrir."
                    }
                },
                "required": ["evento_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "editar_gasto".to_string(),
            description: "Edita los detalles de un gasto financiero registrado.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "gasto_id": {
                        "type": "string",
                        "description": "Identificador único del gasto a editar."
                    },
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
                        ],
                        "description": "Categoría contable del gasto."
                    },
                    "monto": {
                        "type": "number",
                        "minimum": 0.0,
                        "description": "Monto del gasto en soles (S/)."
                    },
                    "descripcion": {
                        "type": "string",
                        "description": "Descripción o detalle del gasto."
                    },
                    "fecha": {
                        "type": "string",
                        "description": "Fecha del gasto en formato YYYY-MM-DD."
                    }
                },
                "required": ["gasto_id", "categoria", "monto", "descripcion", "fecha"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "editar_interesado".to_string(),
            description: "Actualiza los datos y estado de una persona interesada o reserva en un evento.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "interesado_id": {
                        "type": "string",
                        "description": "Identificador único del interesado a editar."
                    },
                    "nombre": {
                        "type": "string",
                        "description": "Nombre de la persona interesada."
                    },
                    "contacto": {
                        "type": "string",
                        "description": "Teléfono o celular de contacto."
                    },
                    "canal": {
                        "type": "string",
                        "description": "Canal de comunicación (ej: WhatsApp, Instagram, Personal)."
                    },
                    "personas": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Número de personas estimadas en el grupo."
                    },
                    "promotora": {
                        "type": "string",
                        "description": "Nombre de la promotora responsable del contacto."
                    },
                    "estado": {
                        "type": "string",
                        "enum": ["Pendiente", "Confirmado", "Cancelado"],
                        "description": "Estado del interesado: 'Pendiente', 'Confirmado' o 'Cancelado'."
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
            description: "Convierte un interesado en venta confirmada emitiendo sus entradas y actualizando su estado a Confirmado.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "interesado_id": {
                        "type": "string",
                        "description": "Identificador único de la persona interesada."
                    },
                    "tipo_entrada": {
                        "type": "string",
                        "description": "Nombre exacto del tipo de entrada a emitir."
                    },
                    "cantidad": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Cantidad de entradas a emitir."
                    },
                    "medio_pago": {
                        "type": "string",
                        "enum": ["Yape", "Efectivo", "Cortesía"],
                        "description": "Medio de pago: 'Yape', 'Efectivo' o 'Cortesía'."
                    },
                    "estado_pago": {
                        "type": "string",
                        "enum": ["Pagado", "Pendiente"],
                        "description": "Estado de la entrada: 'Pagado' o 'Pendiente'."
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
            description: "Elimina permanentemente una presentación y todos sus elementos (canciones, letras y notas). Requiere confirmación previa.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentacion_id": {
                        "type": "string",
                        "description": "Identificador único de la presentación a eliminar."
                    }
                },
                "required": ["presentacion_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "eliminar_elemento".to_string(),
            description: "Elimina permanentemente un elemento (canción, letra o nota) de una presentación. Requiere confirmación previa.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "presentacion_id": {
                        "type": "string",
                        "description": "Identificador único de la presentación."
                    },
                    "elemento_id": {
                        "type": "string",
                        "description": "Identificador único del elemento a eliminar."
                    }
                },
                "required": ["presentacion_id", "elemento_id"],
                "additionalProperties": false
            }),
        },
        // Destructive ticketing tools (Slice 2)
        ToolDefinition {
            name: "eliminar_evento".to_string(),
            description: "Elimina permanentemente un evento y todas sus entradas, gastos, interesados y su afiche. Requiere confirmación previa.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": {
                        "type": "string",
                        "description": "Identificador único del evento a eliminar."
                    }
                },
                "required": ["evento_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "finalizar_evento".to_string(),
            description: "Finaliza un evento cerrando la venta de entradas y bloqueando modificaciones operativas. Requiere confirmación previa.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "evento_id": {
                        "type": "string",
                        "description": "Identificador único del evento a finalizar."
                    }
                },
                "required": ["evento_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "eliminar_entrada".to_string(),
            description: "Anula y elimina definitivamente una entrada emitida. Requiere confirmación previa.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "entrada_id": {
                        "type": "string",
                        "description": "Identificador único de la entrada a eliminar."
                    }
                },
                "required": ["entrada_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "eliminar_gasto".to_string(),
            description: "Elimina permanentemente un registro de gasto de un evento. Requiere confirmación previa.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "gasto_id": {
                        "type": "string",
                        "description": "Identificador único del gasto a eliminar."
                    }
                },
                "required": ["gasto_id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "eliminar_interesado".to_string(),
            description: "Elimina definitivamente un interesado/lead registrado en un evento. Requiere confirmación previa.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "interesado_id": {
                        "type": "string",
                        "description": "Identificador único del interesado a eliminar."
                    }
                },
                "required": ["interesado_id"],
                "additionalProperties": false
            }),
        },
    ]
}
