use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;

use shirley_show::agent::tool_dispatcher::AgentToolDispatcher;
use shirley_show::agent::tool_schemas::get_tool_definitions;
use shirley_show::agent::turn_context::{StagedImageHandle, TurnToolContext};
use shirley_show::domain::agent_tool::ToolCall;
use shirley_show::infra::filesystem::LocalAudioStorage;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
use shirley_show::ports::ticketing::{EventRepository, ImageFormat};
use shirley_show::state::AppState;

fn setup_test_state() -> (AppState, TempDir) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let audio_dir = temp_dir.path().join("audio");
    std::fs::create_dir_all(audio_dir.join("files")).unwrap();
    std::fs::create_dir_all(audio_dir.join("staging")).unwrap();

    let db_path = temp_dir.path().join("test.sqlite3");
    let pool = init_pool(db_path.to_str().unwrap()).unwrap();
    migrate_pool(&pool).unwrap();

    let storage = Arc::new(LocalAudioStorage::new(audio_dir));
    let state = AppState::new_with_items(pool, storage);
    (state, temp_dir)
}

#[test]
fn test_tool_schemas_strictly_sixteen_tools_with_exact_ticketing_schemas() {
    let tools = get_tool_definitions();
    assert_eq!(
        tools.len(),
        32,
        "get_tool_definitions() must expose exactly 32 tools in Slice 3 (13 presentations + 19 ticketing)"
    );

    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    // 8 original presentation tools
    assert!(names.contains(&"create_presentation"));
    assert!(names.contains(&"list_presentations"));
    assert!(names.contains(&"get_presentation"));
    assert!(names.contains(&"add_lyrics"));
    assert!(names.contains(&"add_note"));
    assert!(names.contains(&"add_track_from_staged"));
    assert!(names.contains(&"download_youtube_track"));
    assert!(names.contains(&"separate_vocals"));

    // 8 original ticketing tools
    assert!(names.contains(&"crear_evento"));
    assert!(names.contains(&"listar_eventos"));
    assert!(names.contains(&"vender_entradas"));
    assert!(names.contains(&"confirmar_pago_entrada"));
    assert!(names.contains(&"registrar_gasto"));
    assert!(names.contains(&"resumen_evento"));
    assert!(names.contains(&"registrar_interesado"));
    assert!(names.contains(&"checkin_entrada"));

    // 5 safe ticketing tools (Slice 1)
    assert!(names.contains(&"editar_evento"));
    assert!(names.contains(&"reabrir_evento"));
    assert!(names.contains(&"editar_gasto"));
    assert!(names.contains(&"editar_interesado"));
    assert!(names.contains(&"confirmar_interesado"));

    // 5 destructive ticketing tools (Slice 2)
    assert!(names.contains(&"eliminar_evento"));
    assert!(names.contains(&"finalizar_evento"));
    assert!(names.contains(&"eliminar_entrada"));
    assert!(names.contains(&"eliminar_gasto"));
    assert!(names.contains(&"eliminar_interesado"));

    // 1 poster ticketing tool (Slice 3)
    assert!(names.contains(&"subir_afiche"));

    // Assert architectural exclusions:
    assert!(!names.contains(&"reproducir_audio"));
    assert!(!names.contains(&"play"));
    assert!(!names.contains(&"pause"));
    assert!(!names.contains(&"stage_mode"));
    assert!(!names.contains(&"modo_escenario"));
    assert!(!names.contains(&"camera_scan"));
    assert!(!names.contains(&"escanear_camara"));

    for tool in &tools {
        let params = &tool.parameters;
        assert_eq!(
            params["type"], "object",
            "Tool {} type must be object",
            tool.name
        );
        assert_eq!(
            params["additionalProperties"], false,
            "Tool {} must disallow additionalProperties",
            tool.name
        );

        match tool.name.as_str() {
            "crear_evento" => {
                let required = params["required"]
                    .as_array()
                    .expect("crear_evento required");
                assert!(required.contains(&json!("nombre")));
                assert!(required.contains(&json!("fecha_hora")));
                assert!(required.contains(&json!("lugar")));
                assert!(required.contains(&json!("tipos_de_entrada")));

                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["nombre"]["type"], "string");
                assert_eq!(props["fecha_hora"]["type"], "string");
                assert_eq!(props["lugar"]["type"], "string");
                assert_eq!(props["tipos_de_entrada"]["type"], "array");

                let item_props = props["tipos_de_entrada"]["items"]["properties"]
                    .as_object()
                    .unwrap();
                assert_eq!(item_props["nombre"]["type"], "string");
                assert_eq!(item_props["precio"]["type"], "number");
                assert_eq!(item_props["capacidad"]["type"], "integer");
                assert_eq!(
                    props["tipos_de_entrada"]["items"]["additionalProperties"],
                    false
                );
            }
            "listar_eventos" => {
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["incluir_finalizados"]["type"], "boolean");
                assert!(
                    params
                        .get("required")
                        .is_none_or(|r| r.as_array().unwrap().is_empty())
                );
            }
            "vender_entradas" => {
                let required = params["required"]
                    .as_array()
                    .expect("vender_entradas required");
                assert_eq!(required.len(), 7);
                assert!(required.contains(&json!("evento_id")));
                assert!(required.contains(&json!("nombre_comprador")));
                assert!(required.contains(&json!("contacto")));
                assert!(required.contains(&json!("tipo_entrada")));
                assert!(required.contains(&json!("cantidad")));
                assert!(required.contains(&json!("medio_pago")));
                assert!(required.contains(&json!("estado_pago")));

                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["cantidad"]["type"], "integer");
                assert_eq!(props["cantidad"]["minimum"], 1);

                let medio_enum = props["medio_pago"]["enum"].as_array().unwrap();
                assert!(medio_enum.contains(&json!("Yape")));
                assert!(medio_enum.contains(&json!("Efectivo")));
                assert!(medio_enum.contains(&json!("Cortesía")));

                let estado_enum = props["estado_pago"]["enum"].as_array().unwrap();
                assert!(estado_enum.contains(&json!("Pagado")));
                assert!(estado_enum.contains(&json!("Pendiente")));
            }
            "confirmar_pago_entrada" => {
                let required = params["required"]
                    .as_array()
                    .expect("confirmar_pago required");
                assert_eq!(required.len(), 1);
                assert!(required.contains(&json!("entrada_id")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["entrada_id"]["type"], "string");
            }
            "registrar_gasto" => {
                let required = params["required"]
                    .as_array()
                    .expect("registrar_gasto required");
                assert!(required.contains(&json!("evento_id")));
                assert!(required.contains(&json!("categoria")));
                assert!(required.contains(&json!("monto")));
                assert!(required.contains(&json!("descripcion")));

                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["monto"]["type"], "number");
                assert_eq!(props["monto"]["minimum"], 0.0);

                let cat_enum = props["categoria"]["enum"].as_array().unwrap();
                assert_eq!(cat_enum.len(), 7);
                assert!(cat_enum.contains(&json!("Local/Alquiler")));
                assert!(cat_enum.contains(&json!("Sonido/Luces")));
                assert!(cat_enum.contains(&json!("Músicos/Personal")));
                assert!(cat_enum.contains(&json!("Publicidad/Marketing")));
                assert!(cat_enum.contains(&json!("Bebidas/Catering")));
                assert!(cat_enum.contains(&json!("Transporte")));
                assert!(cat_enum.contains(&json!("Otros")));
            }
            "resumen_evento" => {
                let required = params["required"]
                    .as_array()
                    .expect("resumen_evento required");
                assert_eq!(required.len(), 1);
                assert!(required.contains(&json!("evento_id")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["evento_id"]["type"], "string");
            }
            "registrar_interesado" => {
                let required = params["required"]
                    .as_array()
                    .expect("registrar_interesado required");
                assert_eq!(required.len(), 6);
                assert!(required.contains(&json!("evento_id")));
                assert!(required.contains(&json!("nombre")));
                assert!(required.contains(&json!("contacto")));
                assert!(required.contains(&json!("canal")));
                assert!(required.contains(&json!("personas")));
                assert!(required.contains(&json!("promotora")));

                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["personas"]["type"], "integer");
                assert_eq!(props["personas"]["minimum"], 1);
            }
            "checkin_entrada" => {
                let required = params["required"]
                    .as_array()
                    .expect("checkin_entrada required");
                assert_eq!(required.len(), 2);
                assert!(required.contains(&json!("evento_id")));
                assert!(required.contains(&json!("codigo")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["evento_id"]["type"], "string");
                assert_eq!(props["codigo"]["type"], "string");
            }
            "editar_evento" => {
                let required = params["required"]
                    .as_array()
                    .expect("editar_evento required");
                assert!(required.contains(&json!("evento_id")));
                assert!(required.contains(&json!("nombre")));
                assert!(required.contains(&json!("fecha_hora")));
                assert!(required.contains(&json!("lugar")));
                assert!(required.contains(&json!("tipos_de_entrada")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["evento_id"]["type"], "string");
                assert_eq!(props["nombre"]["type"], "string");
                assert_eq!(props["fecha_hora"]["type"], "string");
                assert_eq!(props["lugar"]["type"], "string");
                assert_eq!(props["tipos_de_entrada"]["type"], "array");
            }
            "reabrir_evento" => {
                let required = params["required"]
                    .as_array()
                    .expect("reabrir_evento required");
                assert_eq!(required.len(), 1);
                assert!(required.contains(&json!("evento_id")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["evento_id"]["type"], "string");
            }
            "editar_gasto" => {
                let required = params["required"]
                    .as_array()
                    .expect("editar_gasto required");
                assert!(required.contains(&json!("gasto_id")));
                assert!(required.contains(&json!("categoria")));
                assert!(required.contains(&json!("monto")));
                assert!(required.contains(&json!("descripcion")));
                assert!(required.contains(&json!("fecha")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["monto"]["type"], "number");
                assert_eq!(props["monto"]["minimum"], 0.0);
                let cat_enum = props["categoria"]["enum"].as_array().unwrap();
                assert!(cat_enum.contains(&json!("Local/Alquiler")));
                assert!(cat_enum.contains(&json!("Sonido/Luces")));
            }
            "editar_interesado" => {
                let required = params["required"]
                    .as_array()
                    .expect("editar_interesado required");
                assert!(required.contains(&json!("interesado_id")));
                assert!(required.contains(&json!("nombre")));
                assert!(required.contains(&json!("contacto")));
                assert!(required.contains(&json!("canal")));
                assert!(required.contains(&json!("personas")));
                assert!(required.contains(&json!("promotora")));
                assert!(required.contains(&json!("estado")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["personas"]["type"], "integer");
                assert_eq!(props["personas"]["minimum"], 1);
                let est_enum = props["estado"]["enum"].as_array().unwrap();
                assert!(est_enum.contains(&json!("Pendiente")));
                assert!(est_enum.contains(&json!("Confirmado")));
                assert!(est_enum.contains(&json!("Cancelado")));
            }
            "confirmar_interesado" => {
                let required = params["required"]
                    .as_array()
                    .expect("confirmar_interesado required");
                assert!(required.contains(&json!("interesado_id")));
                assert!(required.contains(&json!("tipo_entrada")));
                assert!(required.contains(&json!("cantidad")));
                assert!(required.contains(&json!("medio_pago")));
                assert!(required.contains(&json!("estado_pago")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["cantidad"]["type"], "integer");
                assert_eq!(props["cantidad"]["minimum"], 1);
                let mp_enum = props["medio_pago"]["enum"].as_array().unwrap();
                assert!(mp_enum.contains(&json!("Yape")));
                assert!(mp_enum.contains(&json!("Efectivo")));
                assert!(mp_enum.contains(&json!("Cortesía")));
                let ep_enum = props["estado_pago"]["enum"].as_array().unwrap();
                assert!(ep_enum.contains(&json!("Pagado")));
                assert!(ep_enum.contains(&json!("Pendiente")));
            }
            "eliminar_evento" => {
                let required = params["required"]
                    .as_array()
                    .expect("eliminar_evento required");
                assert_eq!(required.len(), 1);
                assert!(required.contains(&json!("evento_id")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["evento_id"]["type"], "string");
            }
            "finalizar_evento" => {
                let required = params["required"]
                    .as_array()
                    .expect("finalizar_evento required");
                assert_eq!(required.len(), 1);
                assert!(required.contains(&json!("evento_id")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["evento_id"]["type"], "string");
            }
            "eliminar_entrada" => {
                let required = params["required"]
                    .as_array()
                    .expect("eliminar_entrada required");
                assert_eq!(required.len(), 1);
                assert!(required.contains(&json!("entrada_id")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["entrada_id"]["type"], "string");
            }
            "eliminar_gasto" => {
                let required = params["required"]
                    .as_array()
                    .expect("eliminar_gasto required");
                assert_eq!(required.len(), 1);
                assert!(required.contains(&json!("gasto_id")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["gasto_id"]["type"], "string");
            }
            "eliminar_interesado" => {
                let required = params["required"]
                    .as_array()
                    .expect("eliminar_interesado required");
                assert_eq!(required.len(), 1);
                assert!(required.contains(&json!("interesado_id")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props["interesado_id"]["type"], "string");
            }
            "subir_afiche" => {
                let required = params["required"]
                    .as_array()
                    .expect("subir_afiche required");
                assert_eq!(required.len(), 1);
                assert!(required.contains(&json!("evento_id")));
                let props = params["properties"].as_object().unwrap();
                assert_eq!(props.len(), 1);
                assert_eq!(props["evento_id"]["type"], "string");
            }
            _ => {}
        }
    }
}

#[tokio::test]
async fn test_dispatcher_rejects_invented_event_id() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);
    let mut context = TurnToolContext::default();

    // 1. vender_entradas with invented event_id
    let sell_call = ToolCall {
        id: "call_tix_inv_1".to_string(),
        name: "vender_entradas".to_string(),
        arguments: json!({
            "evento_id": "evento-inventado-404",
            "nombre_comprador": "Juan Pérez",
            "contacto": "999888777",
            "tipo_entrada": "General",
            "cantidad": 1,
            "medio_pago": "Yape",
            "estado_pago": "Pagado"
        })
        .to_string(),
    };
    let sell_res = dispatcher.dispatch(&sell_call, &mut context).await;
    assert_eq!(
        sell_res["error"], "unauthorized_event_id",
        "Invented event_id must be rejected with unauthorized_event_id"
    );

    // 2. resumen_evento with invented event_id
    let summary_call = ToolCall {
        id: "call_tix_inv_2".to_string(),
        name: "resumen_evento".to_string(),
        arguments: json!({
            "evento_id": "evento-inventado-404"
        })
        .to_string(),
    };
    let summary_res = dispatcher.dispatch(&summary_call, &mut context).await;
    assert_eq!(
        summary_res["error"], "unauthorized_event_id",
        "Invented event_id must be rejected for resumen_evento"
    );

    // 3. checkin_entrada with invented event_id
    let checkin_call = ToolCall {
        id: "call_tix_inv_3".to_string(),
        name: "checkin_entrada".to_string(),
        arguments: json!({
            "evento_id": "evento-inventado-404",
            "codigo": "abcdef0123456789abcdef0123456789"
        })
        .to_string(),
    };
    let checkin_res = dispatcher.dispatch(&checkin_call, &mut context).await;
    assert_eq!(
        checkin_res["error"], "unauthorized_event_id",
        "Invented event_id must be rejected for checkin_entrada"
    );

    // 4. registrar_gasto with invented event_id
    let expense_call = ToolCall {
        id: "call_tix_inv_4".to_string(),
        name: "registrar_gasto".to_string(),
        arguments: json!({
            "evento_id": "evento-inventado-404",
            "categoria": "Sonido/Luces",
            "monto": 100.0,
            "descripcion": "Consola"
        })
        .to_string(),
    };
    let expense_res = dispatcher.dispatch(&expense_call, &mut context).await;
    assert_eq!(expense_res["error"], "unauthorized_event_id");

    // 5. registrar_interesado with invented event_id
    let lead_call = ToolCall {
        id: "call_tix_inv_5".to_string(),
        name: "registrar_interesado".to_string(),
        arguments: json!({
            "evento_id": "evento-inventado-404",
            "nombre": "Ana",
            "contacto": "999111222",
            "canal": "WhatsApp",
            "personas": 2,
            "promotora": "Camila"
        })
        .to_string(),
    };
    let lead_res = dispatcher.dispatch(&lead_call, &mut context).await;
    assert_eq!(lead_res["error"], "unauthorized_event_id");
}

#[tokio::test]
async fn test_dispatcher_eight_ticketing_tools_full_operational_lifecycle() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state.clone())
        .with_public_base_url(Some("https://shirley.show".to_string()));
    let mut context = TurnToolContext::default();

    // 1. crear_evento
    let create_call = ToolCall {
        id: "call_tix_1".to_string(),
        name: "crear_evento".to_string(),
        arguments: json!({
            "nombre": "Shirley Acústico 2026",
            "fecha_hora": "2026-06-20T20:00:00Z",
            "lugar": "Jazz Zone Miraflores",
            "tipos_de_entrada": [
                { "nombre": "General", "precio": 40.0, "capacidad": 80 },
                { "nombre": "VIP", "precio": 70.0, "capacidad": 20 },
                { "nombre": "Cortesía", "precio": 0.0, "capacidad": 5 }
            ]
        })
        .to_string(),
    };
    let create_res = dispatcher.dispatch(&create_call, &mut context).await;
    assert!(
        create_res.get("id").is_some(),
        "crear_evento failed: {create_res}"
    );
    let event_id = create_res["id"].as_str().unwrap().to_string();
    assert_eq!(create_res["nombre"], "Shirley Acústico 2026");
    assert!(context.known_event_ids.contains(&event_id));
    assert_eq!(context.active_event_id, Some(event_id.clone()));

    // 2. listar_eventos
    let list_call = ToolCall {
        id: "call_tix_2".to_string(),
        name: "listar_eventos".to_string(),
        arguments: "{}".to_string(),
    };
    let list_res = dispatcher.dispatch(&list_call, &mut context).await;
    let eventos = list_res["eventos"].as_array().expect("eventos array");
    assert!(eventos.iter().any(|e| e["id"] == event_id));

    // 3. vender_entradas (Pagado)
    let sell_call = ToolCall {
        id: "call_tix_3".to_string(),
        name: "vender_entradas".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "nombre_comprador": "Carlos Mendoza",
            "contacto": "998877665",
            "tipo_entrada": "General",
            "cantidad": 2,
            "medio_pago": "Yape",
            "estado_pago": "Pagado"
        })
        .to_string(),
    };
    let sell_res = dispatcher.dispatch(&sell_call, &mut context).await;
    let entradas = sell_res["entradas"].as_array().expect("entradas array");
    assert_eq!(entradas.len(), 2);
    assert_eq!(sell_res["total_entradas"], 2);

    for entrada in entradas {
        let token = entrada["scan_token"].as_str().unwrap();
        assert!(token.starts_with("st_"));
        let ticket_url = entrada["ticket_url"].as_str().unwrap();
        assert_eq!(ticket_url, format!("https://shirley.show/#/ticket/{token}"));
        let wa = entrada["whatsapp_url"].as_str().unwrap();
        assert!(wa.starts_with("https://wa.me/51998877665?text="));
        assert!(
            context
                .known_ticket_ids
                .contains(entrada["id"].as_str().unwrap())
        );
    }

    // 4. vender_entradas (Pendiente) para luego confirmar
    let sell_pending_call = ToolCall {
        id: "call_tix_4".to_string(),
        name: "vender_entradas".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "nombre_comprador": "Lucía Ramos",
            "contacto": "987654321",
            "tipo_entrada": "VIP",
            "cantidad": 1,
            "medio_pago": "Efectivo",
            "estado_pago": "Pendiente"
        })
        .to_string(),
    };
    let pending_res = dispatcher.dispatch(&sell_pending_call, &mut context).await;
    let pending_ticket = &pending_res["entradas"][0];
    assert_eq!(pending_ticket["estado_pago"], "Pendiente");
    let pending_id = pending_ticket["id"].as_str().unwrap().to_string();

    // 5. confirmar_pago_entrada
    let confirm_call = ToolCall {
        id: "call_tix_5".to_string(),
        name: "confirmar_pago_entrada".to_string(),
        arguments: json!({ "entrada_id": pending_id }).to_string(),
    };
    let confirm_res = dispatcher.dispatch(&confirm_call, &mut context).await;
    assert_eq!(confirm_res["estado_pago"], "Pagado");

    // Rejection of unknown ticket_id
    let unk_ticket_call = ToolCall {
        id: "call_tix_5_unk".to_string(),
        name: "confirmar_pago_entrada".to_string(),
        arguments: json!({ "entrada_id": "ticket-fantasma-999" }).to_string(),
    };
    let unk_ticket_res = dispatcher.dispatch(&unk_ticket_call, &mut context).await;
    assert_eq!(unk_ticket_res["error"], "unauthorized_ticket_id");

    // 6. registrar_gasto
    let expense_call = ToolCall {
        id: "call_tix_6".to_string(),
        name: "registrar_gasto".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "categoria": "Sonido/Luces",
            "monto": 50.0,
            "descripcion": "Consola y monitores",
            "fecha": "2026-06-19"
        })
        .to_string(),
    };
    let expense_res = dispatcher.dispatch(&expense_call, &mut context).await;
    assert_eq!(expense_res["categoria"], "Sonido/Luces");
    assert_eq!(expense_res["monto"], 50.0);

    // 7. registrar_interesado
    let lead_call = ToolCall {
        id: "call_tix_7".to_string(),
        name: "registrar_interesado".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "nombre": "Mariana Costa",
            "contacto": "912345678",
            "canal": "Instagram",
            "personas": 4,
            "promotora": "Camila"
        })
        .to_string(),
    };
    let lead_res = dispatcher.dispatch(&lead_call, &mut context).await;
    assert_eq!(lead_res["personas"], 4);
    assert_eq!(lead_res["nombre"], "Mariana Costa");

    // 8. resumen_evento
    let summary_call = ToolCall {
        id: "call_tix_8".to_string(),
        name: "resumen_evento".to_string(),
        arguments: json!({ "evento_id": event_id }).to_string(),
    };
    let summary_res = dispatcher.dispatch(&summary_call, &mut context).await;
    // 2 General @ 40 = 80, 1 VIP @ 70 = 70 => total cobrado = 150.0
    assert_eq!(summary_res["total_cobrado"], 150.0);
    assert_eq!(summary_res["total_gastos"], 50.0);
    assert_eq!(summary_res["ganancia_neta"], 100.0); // 10000 cents = S/ 100.0
    assert_eq!(summary_res["entradas_emitidas"], 3);

    // 9. checkin_entrada
    // Retrieve secret codigo_qr from DB for the first ticket
    let pool = state.pool.clone();
    let first_ticket_id = entradas[0]["id"].as_str().unwrap().to_string();
    let qr_code: String = {
        let conn = pool.get().unwrap();
        conn.query_row(
            "SELECT codigo_qr FROM entradas WHERE id = ?1",
            [&first_ticket_id],
            |r| r.get(0),
        )
        .unwrap()
    };

    let checkin_call = ToolCall {
        id: "call_tix_9".to_string(),
        name: "checkin_entrada".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "codigo": qr_code
        })
        .to_string(),
    };
    let checkin_res = dispatcher.dispatch(&checkin_call, &mut context).await;
    assert_eq!(checkin_res["resultado"], "Valida");

    // Second check-in (e.g. in subsequent turn) must return Ya Usada
    let mut context_turn2 = TurnToolContext::default();
    context_turn2.known_event_ids.insert(event_id.clone());
    let checkin_again_call = ToolCall {
        id: "call_tix_9_again".to_string(),
        name: "checkin_entrada".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "codigo": qr_code
        })
        .to_string(),
    };
    let checkin_again_res = dispatcher
        .dispatch(&checkin_again_call, &mut context_turn2)
        .await;
    assert_eq!(checkin_again_res["resultado"], "Ya Usada");

    // Invalid QR code returns Invalida
    let invalid_checkin_call = ToolCall {
        id: "call_tix_9_invalid".to_string(),
        name: "checkin_entrada".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "codigo": "0123456789abcdef0123456789abcdef"
        })
        .to_string(),
    };
    let invalid_res = dispatcher
        .dispatch(&invalid_checkin_call, &mut context)
        .await;
    assert_eq!(invalid_res["resultado"], "Invalida");
}

#[tokio::test]
async fn test_dispatcher_triangulate_deduplication_by_call_id() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state.clone());
    let mut context = TurnToolContext::default();

    // Create event first
    let create_call = ToolCall {
        id: "call_dedup_setup".to_string(),
        name: "crear_evento".to_string(),
        arguments: json!({
            "nombre": "Evento Dedup",
            "fecha_hora": "2026-07-01T21:00:00Z",
            "lugar": "Barranco",
            "tipos_de_entrada": [
                { "nombre": "General", "precio": 30.0, "capacidad": 50 }
            ]
        })
        .to_string(),
    };
    let create_res = dispatcher.dispatch(&create_call, &mut context).await;
    let event_id = create_res["id"].as_str().unwrap();

    // 1. Mutating tool: duplicate call.id ignored
    let sell_call = ToolCall {
        id: "call_sale_dedup_1".to_string(),
        name: "vender_entradas".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "nombre_comprador": "Pedro Ruiz",
            "contacto": "999333444",
            "tipo_entrada": "General",
            "cantidad": 1,
            "medio_pago": "Yape",
            "estado_pago": "Pagado"
        })
        .to_string(),
    };
    let res1 = dispatcher.dispatch(&sell_call, &mut context).await;
    assert_eq!(res1["total_entradas"], 1);

    // Re-dispatch same call.id
    let res2 = dispatcher.dispatch(&sell_call, &mut context).await;
    assert_eq!(res2["warning"], "duplicate_call_ignored");

    // Verify DB only has 1 ticket
    let ticket_count: i64 = {
        let conn = state.pool.get().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM entradas WHERE evento_id = ?1",
            [event_id],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(
        ticket_count, 1,
        "Must NOT emit duplicate tickets on repeated call.id"
    );

    // 2. Read tool: duplicate call.id returns cached value
    let list_call = ToolCall {
        id: "call_list_dedup_1".to_string(),
        name: "listar_eventos".to_string(),
        arguments: "{}".to_string(),
    };
    let list_res1 = dispatcher.dispatch(&list_call, &mut context).await;
    let list_res2 = dispatcher.dispatch(&list_call, &mut context).await;
    assert_eq!(list_res1, list_res2);
}

#[tokio::test]
async fn test_dispatcher_triangulate_courtesy_forced_zero_price_and_pagado() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state.clone());
    let mut context = TurnToolContext::default();

    // Create event with Cortesía type
    let create_call = ToolCall {
        id: "call_cortesia_setup".to_string(),
        name: "crear_evento".to_string(),
        arguments: json!({
            "nombre": "Show Acústico Cortesías",
            "fecha_hora": "2026-08-10T20:00:00Z",
            "lugar": "Auditorio",
            "tipos_de_entrada": [
                { "nombre": "Cortesía", "precio": 0.0, "capacidad": 10 }
            ]
        })
        .to_string(),
    };
    let create_res = dispatcher.dispatch(&create_call, &mut context).await;
    let event_id = create_res["id"].as_str().unwrap();

    // Sell Cortesía, even if caller specifies "Pendiente"
    let courtesy_call = ToolCall {
        id: "call_cortesia_sell".to_string(),
        name: "vender_entradas".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "nombre_comprador": "Invitado Especial Roberto",
            "contacto": "999000111",
            "tipo_entrada": "Cortesía",
            "cantidad": 2,
            "medio_pago": "Cortesía",
            "estado_pago": "Pendiente"
        })
        .to_string(),
    };
    let res = dispatcher.dispatch(&courtesy_call, &mut context).await;
    let entradas = res["entradas"].as_array().unwrap();
    assert_eq!(entradas.len(), 2);

    for e in entradas {
        assert_eq!(e["precio"], 0.0, "Cortesía price must be forced to 0.0");
        assert_eq!(
            e["estado_pago"], "Pagado",
            "Cortesía status must be forced to Pagado"
        );
        assert_eq!(e["medio_pago"], "Cortesía");
    }

    // Verify DB integrity
    let conn = state.pool.get().unwrap();
    let (precio_centimos, estado_pago): (i64, String) = conn
        .query_row(
            "SELECT precio_centimos, estado_pago FROM entradas WHERE evento_id = ?1 LIMIT 1",
            [event_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(precio_centimos, 0);
    assert_eq!(estado_pago, "Pagado");
}

#[tokio::test]
async fn test_dispatcher_triangulate_ticket_url_formatting_with_public_base_url() {
    let (state, _tmp) = setup_test_state();
    let mut context = TurnToolContext::default();

    // Setup event
    let setup_dispatcher = AgentToolDispatcher::new(state.clone());
    let create_call = ToolCall {
        id: "call_url_setup".to_string(),
        name: "crear_evento".to_string(),
        arguments: json!({
            "nombre": "Evento URLs",
            "fecha_hora": "2026-09-01T20:00:00Z",
            "lugar": "Centro Cultural",
            "tipos_de_entrada": [
                { "nombre": "General", "precio": 25.0, "capacidad": 100 }
            ]
        })
        .to_string(),
    };
    let create_res = setup_dispatcher.dispatch(&create_call, &mut context).await;
    let event_id = create_res["id"].as_str().unwrap();

    // 1. Configured base URL
    let dispatcher_with_url = AgentToolDispatcher::new(state.clone())
        .with_public_base_url(Some("https://shirley.show".to_string()));
    let sell_call_1 = ToolCall {
        id: "call_url_sell_1".to_string(),
        name: "vender_entradas".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "nombre_comprador": "Comprador 1",
            "contacto": "999111000",
            "tipo_entrada": "General",
            "cantidad": 1,
            "medio_pago": "Yape",
            "estado_pago": "Pagado"
        })
        .to_string(),
    };
    let res1 = dispatcher_with_url
        .dispatch(&sell_call_1, &mut context)
        .await;
    let token1 = res1["entradas"][0]["scan_token"].as_str().unwrap();
    assert_eq!(
        res1["entradas"][0]["ticket_url"],
        format!("https://shirley.show/#/ticket/{token1}")
    );

    // 2. Configured base URL with trailing slash: must not double slash
    let dispatcher_with_trailing = AgentToolDispatcher::new(state.clone())
        .with_public_base_url(Some("https://shirley.show/".to_string()));
    let sell_call_2 = ToolCall {
        id: "call_url_sell_2".to_string(),
        name: "vender_entradas".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "nombre_comprador": "Comprador 2",
            "contacto": "999222000",
            "tipo_entrada": "General",
            "cantidad": 1,
            "medio_pago": "Yape",
            "estado_pago": "Pagado"
        })
        .to_string(),
    };
    let res2 = dispatcher_with_trailing
        .dispatch(&sell_call_2, &mut context)
        .await;
    let token2 = res2["entradas"][0]["scan_token"].as_str().unwrap();
    assert_eq!(
        res2["entradas"][0]["ticket_url"],
        format!("https://shirley.show/#/ticket/{token2}")
    );

    // 3. No base URL (dev fallback): relative path
    let dispatcher_no_url = AgentToolDispatcher::new(state.clone()).with_public_base_url(None);
    let sell_call_3 = ToolCall {
        id: "call_url_sell_3".to_string(),
        name: "vender_entradas".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "nombre_comprador": "Comprador 3",
            "contacto": "999333000",
            "tipo_entrada": "General",
            "cantidad": 1,
            "medio_pago": "Yape",
            "estado_pago": "Pagado"
        })
        .to_string(),
    };
    let res3 = dispatcher_no_url.dispatch(&sell_call_3, &mut context).await;
    let token3 = res3["entradas"][0]["scan_token"].as_str().unwrap();
    assert_eq!(
        res3["entradas"][0]["ticket_url"],
        format!("/#/ticket/{token3}")
    );
}

#[tokio::test]
async fn test_dispatcher_safe_ticketing_tools_input_validations() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);
    let mut context = TurnToolContext::default();
    context.known_event_ids.insert("evt-1".to_string());

    // 1. editar_evento: empty tipos_de_entrada
    let edit_event_empty_tiers = ToolCall {
        id: "call_ee_val_1".to_string(),
        name: "editar_evento".to_string(),
        arguments: json!({
            "evento_id": "evt-1",
            "nombre": "Evento Edit",
            "fecha_hora": "2026-10-10T20:00",
            "lugar": "Teatro",
            "tipos_de_entrada": []
        })
        .to_string(),
    };
    let res = dispatcher
        .dispatch(&edit_event_empty_tiers, &mut context)
        .await;
    assert_eq!(res["error"], "validation_error");

    // 2. editar_evento: negative price
    let edit_event_neg_price = ToolCall {
        id: "call_ee_val_2".to_string(),
        name: "editar_evento".to_string(),
        arguments: json!({
            "evento_id": "evt-1",
            "nombre": "Evento Edit",
            "fecha_hora": "2026-10-10T20:00",
            "lugar": "Teatro",
            "tipos_de_entrada": [
                { "nombre": "General", "precio": -10.0, "capacidad": 50 }
            ]
        })
        .to_string(),
    };
    let res = dispatcher
        .dispatch(&edit_event_neg_price, &mut context)
        .await;
    assert_eq!(res["error"], "validation_error");

    // 2b. editar_evento: invalid fecha_hora (not ISO 8601 YYYY-MM-DDTHH:MM)
    let edit_event_bad_date = ToolCall {
        id: "call_ee_val_bad_date".to_string(),
        name: "editar_evento".to_string(),
        arguments: json!({
            "evento_id": "evt-1",
            "nombre": "Evento Edit",
            "fecha_hora": "2026-10-10", // missing time
            "lugar": "Teatro",
            "tipos_de_entrada": [
                { "nombre": "General", "precio": 30.0, "capacidad": 50 }
            ]
        })
        .to_string(),
    };
    let res = dispatcher
        .dispatch(&edit_event_bad_date, &mut context)
        .await;
    assert_eq!(res["error"], "validation_error");

    // 2c. crear_evento: invalid fecha_hora
    let create_event_bad_date = ToolCall {
        id: "call_ce_val_bad_date".to_string(),
        name: "crear_evento".to_string(),
        arguments: json!({
            "nombre": "Evento Nuevo",
            "fecha_hora": "mañana a las 8",
            "lugar": "Teatro",
            "tipos_de_entrada": [
                { "nombre": "General", "precio": 30.0, "capacidad": 50 }
            ]
        })
        .to_string(),
    };
    let res = dispatcher
        .dispatch(&create_event_bad_date, &mut context)
        .await;
    assert_eq!(res["error"], "validation_error");

    // 3. editar_gasto: invalid closed enum categoria
    let edit_expense_bad_cat = ToolCall {
        id: "call_eg_val_1".to_string(),
        name: "editar_gasto".to_string(),
        arguments: json!({
            "gasto_id": "exp-1",
            "categoria": "CategoriaInvalida",
            "monto": 50.0,
            "descripcion": "Gasto test",
            "fecha": "2026-10-10"
        })
        .to_string(),
    };
    let res = dispatcher
        .dispatch(&edit_expense_bad_cat, &mut context)
        .await;
    assert_eq!(res["error"], "validation_error");

    // 4. editar_gasto: negative monto
    let edit_expense_neg_monto = ToolCall {
        id: "call_eg_val_2".to_string(),
        name: "editar_gasto".to_string(),
        arguments: json!({
            "gasto_id": "exp-1",
            "categoria": "Otros",
            "monto": -50.0,
            "descripcion": "Gasto negativo",
            "fecha": "2026-10-10"
        })
        .to_string(),
    };
    let res = dispatcher
        .dispatch(&edit_expense_neg_monto, &mut context)
        .await;
    assert_eq!(res["error"], "validation_error");

    // 4b. editar_gasto: invalid fecha format (must be YYYY-MM-DD)
    let edit_expense_bad_date = ToolCall {
        id: "call_eg_val_bad_date".to_string(),
        name: "editar_gasto".to_string(),
        arguments: json!({
            "gasto_id": "exp-1",
            "categoria": "Otros",
            "monto": 50.0,
            "descripcion": "Gasto fecha invalida",
            "fecha": "2026-10-10T15:30"
        })
        .to_string(),
    };
    let res = dispatcher
        .dispatch(&edit_expense_bad_date, &mut context)
        .await;
    assert_eq!(res["error"], "validation_error");

    // 5. editar_interesado: invalid estado enum
    let edit_interest_bad_status = ToolCall {
        id: "call_ei_val_1".to_string(),
        name: "editar_interesado".to_string(),
        arguments: json!({
            "interesado_id": "int-1",
            "nombre": "Interesado 1",
            "contacto": "999111222",
            "canal": "WhatsApp",
            "personas": 2,
            "promotora": "Valeria",
            "estado": "EstadoInventado"
        })
        .to_string(),
    };
    let res = dispatcher
        .dispatch(&edit_interest_bad_status, &mut context)
        .await;
    assert_eq!(res["error"], "validation_error");

    // 6. editar_interesado: personas 0
    let edit_interest_zero_persons = ToolCall {
        id: "call_ei_val_2".to_string(),
        name: "editar_interesado".to_string(),
        arguments: json!({
            "interesado_id": "int-1",
            "nombre": "Interesado 1",
            "contacto": "999111222",
            "canal": "WhatsApp",
            "personas": 0,
            "promotora": "Valeria",
            "estado": "Pendiente"
        })
        .to_string(),
    };
    let res = dispatcher
        .dispatch(&edit_interest_zero_persons, &mut context)
        .await;
    assert_eq!(res["error"], "validation_error");

    // 7. confirmar_interesado: invalid medio_pago
    let confirm_bad_payment = ToolCall {
        id: "call_ci_val_1".to_string(),
        name: "confirmar_interesado".to_string(),
        arguments: json!({
            "interesado_id": "int-1",
            "tipo_entrada": "General",
            "cantidad": 2,
            "medio_pago": "Bitcoin",
            "estado_pago": "Pagado"
        })
        .to_string(),
    };
    let res = dispatcher
        .dispatch(&confirm_bad_payment, &mut context)
        .await;
    assert_eq!(res["error"], "validation_error");

    // 8. confirmar_interesado: invalid estado_pago
    let confirm_bad_status = ToolCall {
        id: "call_ci_val_2".to_string(),
        name: "confirmar_interesado".to_string(),
        arguments: json!({
            "interesado_id": "int-1",
            "tipo_entrada": "General",
            "cantidad": 2,
            "medio_pago": "Yape",
            "estado_pago": "Fiado"
        })
        .to_string(),
    };
    let res = dispatcher.dispatch(&confirm_bad_status, &mut context).await;
    assert_eq!(res["error"], "validation_error");

    // 9. confirmar_interesado: cantidad 0
    let confirm_zero_qty = ToolCall {
        id: "call_ci_val_3".to_string(),
        name: "confirmar_interesado".to_string(),
        arguments: json!({
            "interesado_id": "int-1",
            "tipo_entrada": "General",
            "cantidad": 0,
            "medio_pago": "Yape",
            "estado_pago": "Pagado"
        })
        .to_string(),
    };
    let res = dispatcher.dispatch(&confirm_zero_qty, &mut context).await;
    assert_eq!(res["error"], "validation_error");
}

#[tokio::test]
async fn test_dispatcher_safe_ticketing_tools_operational_lifecycle() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state.clone())
        .with_public_base_url(Some("https://tickets.shirley.pe".to_string()));
    let mut context = TurnToolContext::default();

    // 1. Setup event via crear_evento
    let create_call = ToolCall {
        id: "call_evt_create".to_string(),
        name: "crear_evento".to_string(),
        arguments: json!({
            "nombre": "Gran Peña Criolla",
            "fecha_hora": "2026-11-20T21:00",
            "lugar": "Centro de Convenciones",
            "tipos_de_entrada": [
                { "nombre": "General", "precio": 35.0, "capacidad": 100 },
                { "nombre": "VIP", "precio": 60.0, "capacidad": 30 }
            ]
        })
        .to_string(),
    };
    let create_res = dispatcher.dispatch(&create_call, &mut context).await;
    let event_id = create_res["id"].as_str().unwrap().to_string();

    // 2. editar_evento
    let edit_event_call = ToolCall {
        id: "call_ee_op".to_string(),
        name: "editar_evento".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "nombre": "Gran Peña Criolla 2026 - Renovada",
            "fecha_hora": "2026-11-20T22:00",
            "lugar": "Centro de Convenciones Lima",
            "tipos_de_entrada": [
                { "nombre": "General", "precio": 40.0, "capacidad": 120 },
                { "nombre": "VIP", "precio": 70.0, "capacidad": 40 }
            ]
        })
        .to_string(),
    };
    let edit_evt_res = dispatcher.dispatch(&edit_event_call, &mut context).await;
    assert_eq!(edit_evt_res["id"], event_id);
    assert_eq!(edit_evt_res["nombre"], "Gran Peña Criolla 2026 - Renovada");
    assert_eq!(edit_evt_res["lugar"], "Centro de Convenciones Lima");
    assert_eq!(context.actions_taken.last().unwrap().tool, "editar_evento");
    assert_eq!(context.actions_taken.last().unwrap().status, "success");
    assert_eq!(
        context.actions_taken.last().unwrap().event_id,
        Some(event_id.clone())
    );

    // 3. Register expense then editar_gasto
    let reg_exp_call = ToolCall {
        id: "call_rg_op".to_string(),
        name: "registrar_gasto".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "categoria": "Otros",
            "monto": 50.0,
            "descripcion": "Cables de audio"
        })
        .to_string(),
    };
    let reg_exp_res = dispatcher.dispatch(&reg_exp_call, &mut context).await;
    let expense_id = reg_exp_res["id"].as_str().unwrap().to_string();

    let edit_exp_call = ToolCall {
        id: "call_eg_op".to_string(),
        name: "editar_gasto".to_string(),
        arguments: json!({
            "gasto_id": expense_id,
            "categoria": "Sonido/Luces",
            "monto": 75.0,
            "descripcion": "Cables de audio balanceados y micrófonos",
            "fecha": "2026-10-15"
        })
        .to_string(),
    };
    let edit_exp_res = dispatcher.dispatch(&edit_exp_call, &mut context).await;
    assert_eq!(edit_exp_res["id"], expense_id);
    assert_eq!(edit_exp_res["categoria"], "Sonido/Luces");
    assert_eq!(edit_exp_res["monto"], 75.0);
    assert_eq!(context.actions_taken.last().unwrap().tool, "editar_gasto");
    assert_eq!(context.actions_taken.last().unwrap().status, "success");
    assert_eq!(
        context.actions_taken.last().unwrap().event_id,
        Some(event_id.clone())
    );

    // 4. Register interest then editar_interesado
    let reg_int_call = ToolCall {
        id: "call_ri_op".to_string(),
        name: "registrar_interesado".to_string(),
        arguments: json!({
            "evento_id": event_id,
            "nombre": "Ana Perez",
            "contacto": "999888777",
            "canal": "Instagram",
            "personas": 2,
            "promotora": "Valeria"
        })
        .to_string(),
    };
    let reg_int_res = dispatcher.dispatch(&reg_int_call, &mut context).await;
    let lead_id = reg_int_res["id"].as_str().unwrap().to_string();

    let edit_int_call = ToolCall {
        id: "call_ei_op".to_string(),
        name: "editar_interesado".to_string(),
        arguments: json!({
            "interesado_id": lead_id,
            "nombre": "Ana Perez Gomez",
            "contacto": "999888777",
            "canal": "WhatsApp",
            "personas": 3,
            "promotora": "Valeria",
            "estado": "Pendiente"
        })
        .to_string(),
    };
    let edit_int_res = dispatcher.dispatch(&edit_int_call, &mut context).await;
    assert_eq!(edit_int_res["id"], lead_id);
    assert_eq!(edit_int_res["nombre"], "Ana Perez Gomez");
    assert_eq!(edit_int_res["personas"], 3);
    assert_eq!(
        context.actions_taken.last().unwrap().tool,
        "editar_interesado"
    );
    assert_eq!(context.actions_taken.last().unwrap().status, "success");
    assert_eq!(
        context.actions_taken.last().unwrap().event_id,
        Some(event_id.clone())
    );

    // 5. confirmar_interesado: atomically converts lead into sale
    let conf_int_call = ToolCall {
        id: "call_ci_op".to_string(),
        name: "confirmar_interesado".to_string(),
        arguments: json!({
            "interesado_id": lead_id,
            "tipo_entrada": "General",
            "cantidad": 3,
            "medio_pago": "Yape",
            "estado_pago": "Pagado"
        })
        .to_string(),
    };
    let conf_int_res = dispatcher.dispatch(&conf_int_call, &mut context).await;
    assert_eq!(conf_int_res["interesado_id"], lead_id);
    assert_eq!(conf_int_res["tickets_creados"], 3);
    let tickets = conf_int_res["tickets"].as_array().unwrap();
    assert_eq!(tickets.len(), 3);
    assert!(
        tickets[0]["ticket_url"]
            .as_str()
            .unwrap()
            .contains("https://tickets.shirley.pe/#/ticket/")
    );
    assert_eq!(
        context.actions_taken.last().unwrap().tool,
        "confirmar_interesado"
    );
    assert_eq!(context.actions_taken.last().unwrap().status, "success");
    assert_eq!(
        context.actions_taken.last().unwrap().event_id,
        Some(event_id.clone())
    );

    // 6. Finalize event directly via use case, then reabrir_evento via dispatcher
    state.ticketing.finalize_event.execute(&event_id).unwrap();

    let reopen_call = ToolCall {
        id: "call_reopen_op".to_string(),
        name: "reabrir_evento".to_string(),
        arguments: json!({
            "evento_id": event_id
        })
        .to_string(),
    };
    let reopen_res = dispatcher.dispatch(&reopen_call, &mut context).await;
    assert_eq!(reopen_res["id"], event_id);
    assert_eq!(reopen_res["estado"], "Activo");
    assert_eq!(context.actions_taken.last().unwrap().tool, "reabrir_evento");
    assert_eq!(context.actions_taken.last().unwrap().status, "success");
    assert_eq!(
        context.actions_taken.last().unwrap().event_id,
        Some(event_id.clone())
    );

    // 7. Safe tool turn deduplication
    let reopen_dup = ToolCall {
        id: "call_reopen_dup".to_string(),
        name: "reabrir_evento".to_string(),
        arguments: json!({
            "evento_id": event_id
        })
        .to_string(),
    };
    let reopen_dup_res = dispatcher.dispatch(&reopen_dup, &mut context).await;
    assert_eq!(reopen_dup_res["warning"], "duplicate_call_ignored");
}

#[tokio::test]
async fn test_dispatcher_safe_ticketing_tools_domain_errors() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);
    let mut context = TurnToolContext::default();

    // 1a. Unauthorized event on editar_evento
    let edit_event_unauth = ToolCall {
        id: "call_ee_unauth".to_string(),
        name: "editar_evento".to_string(),
        arguments: json!({
            "evento_id": "evt-unauth-999",
            "nombre": "Nombre",
            "fecha_hora": "2026-10-10T20:00",
            "lugar": "Lugar",
            "tipos_de_entrada": [
                { "nombre": "General", "precio": 30.0, "capacidad": 50 }
            ]
        })
        .to_string(),
    };
    let res = dispatcher.dispatch(&edit_event_unauth, &mut context).await;
    assert_eq!(res["error"], "unauthorized_event_id");

    // 1b. Non-existent event on editar_evento (authorized in context)
    context
        .known_event_ids
        .insert("evt-non-existent-999".to_string());
    let edit_event_missing = ToolCall {
        id: "call_ee_err".to_string(),
        name: "editar_evento".to_string(),
        arguments: json!({
            "evento_id": "evt-non-existent-999",
            "nombre": "Nombre",
            "fecha_hora": "2026-10-10T20:00",
            "lugar": "Lugar",
            "tipos_de_entrada": [
                { "nombre": "General", "precio": 30.0, "capacidad": 50 }
            ]
        })
        .to_string(),
    };
    let res = dispatcher.dispatch(&edit_event_missing, &mut context).await;
    assert_eq!(res["error"], "event_not_found");

    // 2. Non-existent expense on editar_gasto
    let edit_exp_missing = ToolCall {
        id: "call_eg_err".to_string(),
        name: "editar_gasto".to_string(),
        arguments: json!({
            "gasto_id": "exp-non-existent-999",
            "categoria": "Otros",
            "monto": 20.0,
            "descripcion": "Desc",
            "fecha": "2026-10-10"
        })
        .to_string(),
    };
    let res = dispatcher.dispatch(&edit_exp_missing, &mut context).await;
    assert_eq!(res["error"], "expense_not_found");

    // 3. Non-existent lead on editar_interesado
    let edit_int_missing = ToolCall {
        id: "call_ei_err".to_string(),
        name: "editar_interesado".to_string(),
        arguments: json!({
            "interesado_id": "int-non-existent-999",
            "nombre": "Nombre",
            "contacto": "999888777",
            "canal": "WhatsApp",
            "personas": 2,
            "promotora": "Valeria",
            "estado": "Pendiente"
        })
        .to_string(),
    };
    let res = dispatcher.dispatch(&edit_int_missing, &mut context).await;
    assert_eq!(res["error"], "interest_not_found");

    // 4. Non-existent lead on confirmar_interesado
    let conf_int_missing = ToolCall {
        id: "call_ci_err".to_string(),
        name: "confirmar_interesado".to_string(),
        arguments: json!({
            "interesado_id": "int-non-existent-999",
            "tipo_entrada": "General",
            "cantidad": 2,
            "medio_pago": "Yape",
            "estado_pago": "Pagado"
        })
        .to_string(),
    };
    let res = dispatcher.dispatch(&conf_int_missing, &mut context).await;
    assert_eq!(res["error"], "interest_not_found");

    // 5. Non-existent event on reabrir_evento
    let reopen_missing = ToolCall {
        id: "call_re_err".to_string(),
        name: "reabrir_evento".to_string(),
        arguments: json!({ "evento_id": "evt-non-existent-999" }).to_string(),
    };
    let res = dispatcher.dispatch(&reopen_missing, &mut context).await;
    assert_eq!(res["error"], "event_not_found");

    // Assert that all 5 safe ticketing tools recorded a ToolAction with status="error" and affected IDs
    let error_actions: Vec<_> = context
        .actions_taken
        .iter()
        .filter(|a| a.status == "error")
        .collect();
    assert!(
        error_actions
            .iter()
            .any(|a| a.tool == "editar_evento"
                && a.event_id == Some("evt-non-existent-999".to_string())),
        "Missing error ToolAction for editar_evento"
    );
    assert!(
        error_actions.iter().any(|a| a.tool == "reabrir_evento"
            && a.event_id == Some("evt-non-existent-999".to_string())),
        "Missing error ToolAction for reabrir_evento"
    );
    assert!(
        error_actions.iter().any(|a| a.tool == "editar_gasto"),
        "Missing error ToolAction for editar_gasto"
    );
    assert!(
        error_actions.iter().any(|a| a.tool == "editar_interesado"),
        "Missing error ToolAction for editar_interesado"
    );
    assert!(
        error_actions
            .iter()
            .any(|a| a.tool == "confirmar_interesado"),
        "Missing error ToolAction for confirmar_interesado"
    );
}

#[tokio::test]
async fn test_dispatcher_subir_afiche_successful_replacement() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state.clone());

    let create_call = ToolCall {
        id: "call_ev_afiche".to_string(),
        name: "crear_evento".to_string(),
        arguments: json!({
            "nombre": "Show con Afiche",
            "fecha_hora": "2026-11-20T21:00",
            "lugar": "Centro Cultural",
            "tipos_de_entrada": [{ "nombre": "General", "precio": 30.0, "capacidad": 100 }]
        })
        .to_string(),
    };
    let mut setup_context = TurnToolContext::default();
    let create_res = dispatcher.dispatch(&create_call, &mut setup_context).await;
    let event_id = create_res["id"].as_str().unwrap().to_string();

    let mut context =
        TurnToolContext::with_event_context(None, Some(event_id.clone()), None, None, None);
    let valid_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
    context.set_staged_image(Some(StagedImageHandle::Available {
        data: valid_jpeg.clone(),
        format: ImageFormat::Jpeg,
        byte_size: valid_jpeg.len(),
    }));

    let afiche_call = ToolCall {
        id: "call_subir_1".to_string(),
        name: "subir_afiche".to_string(),
        arguments: json!({ "evento_id": event_id }).to_string(),
    };

    let result = dispatcher.dispatch(&afiche_call, &mut context).await;
    assert!(result.get("error").is_none(), "Must succeed: {:?}", result);
    assert_eq!(result["evento_id"], event_id);
    assert!(result["filename"].is_string());
    assert_eq!(result["format"], "jpg");
    assert_eq!(result["byte_size"], valid_jpeg.len());

    // Check handle consumed
    match context.staged_image {
        Some(StagedImageHandle::Consumed) => {}
        _ => panic!("staged_image must be in Consumed state"),
    }

    // Check action recorded
    let action = context
        .actions_taken
        .iter()
        .find(|a| a.tool == "subir_afiche")
        .expect("ToolAction for subir_afiche must be recorded");
    assert_eq!(action.status, "success");
    assert_eq!(action.event_id.as_deref(), Some(event_id.as_str()));

    // Check event updated in repository
    let event = state
        .ticketing
        .repo
        .get_event(&event_id)
        .unwrap()
        .expect("Event must exist");
    assert!(event.imagen.is_some());
}

#[tokio::test]
async fn test_dispatcher_subir_afiche_missing_image_returns_no_image_staged() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);

    let create_call = ToolCall {
        id: "call_ev_no_img".to_string(),
        name: "crear_evento".to_string(),
        arguments: json!({
            "nombre": "Show Sin Afiche",
            "fecha_hora": "2026-11-20T21:00",
            "lugar": "Centro Cultural",
            "tipos_de_entrada": [{ "nombre": "General", "precio": 30.0, "capacidad": 100 }]
        })
        .to_string(),
    };
    let mut setup_context = TurnToolContext::default();
    let create_res = dispatcher.dispatch(&create_call, &mut setup_context).await;
    let event_id = create_res["id"].as_str().unwrap().to_string();

    let mut context =
        TurnToolContext::with_event_context(None, Some(event_id.clone()), None, None, None);
    // context.staged_image is None

    let afiche_call = ToolCall {
        id: "call_subir_missing".to_string(),
        name: "subir_afiche".to_string(),
        arguments: json!({ "evento_id": event_id }).to_string(),
    };

    let result = dispatcher.dispatch(&afiche_call, &mut context).await;
    assert_eq!(result["error"], "no_image_staged");
}

#[tokio::test]
async fn test_dispatcher_subir_afiche_nonexistent_event_returns_evento_not_found() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);

    let mut context = TurnToolContext::default();
    let valid_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    context.set_staged_image(Some(StagedImageHandle::Available {
        data: valid_jpeg.clone(),
        format: ImageFormat::Jpeg,
        byte_size: valid_jpeg.len(),
    }));

    let afiche_call = ToolCall {
        id: "call_subir_ghost".to_string(),
        name: "subir_afiche".to_string(),
        arguments: json!({ "evento_id": "evento-ghost-999" }).to_string(),
    };

    let result = dispatcher.dispatch(&afiche_call, &mut context).await;
    assert_eq!(result["error"], "evento_not_found");
}

#[tokio::test]
async fn test_dispatcher_subir_afiche_duplicate_call_rejected_image_already_consumed() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state);

    let create_call = ToolCall {
        id: "call_ev_dup_afiche".to_string(),
        name: "crear_evento".to_string(),
        arguments: json!({
            "nombre": "Show Duplicado",
            "fecha_hora": "2026-11-20T21:00",
            "lugar": "Centro Cultural",
            "tipos_de_entrada": [{ "nombre": "General", "precio": 30.0, "capacidad": 100 }]
        })
        .to_string(),
    };
    let mut setup_context = TurnToolContext::default();
    let create_res = dispatcher.dispatch(&create_call, &mut setup_context).await;
    let event_id = create_res["id"].as_str().unwrap().to_string();

    let mut context =
        TurnToolContext::with_event_context(None, Some(event_id.clone()), None, None, None);
    let valid_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
    context.set_staged_image(Some(StagedImageHandle::Available {
        data: valid_jpeg.clone(),
        format: ImageFormat::Jpeg,
        byte_size: valid_jpeg.len(),
    }));

    let call1 = ToolCall {
        id: "call_subir_dup_1".to_string(),
        name: "subir_afiche".to_string(),
        arguments: json!({ "evento_id": event_id }).to_string(),
    };
    let res1 = dispatcher.dispatch(&call1, &mut context).await;
    assert!(res1.get("error").is_none());

    let call2 = ToolCall {
        id: "call_subir_dup_2".to_string(),
        name: "subir_afiche".to_string(),
        arguments: json!({ "evento_id": event_id }).to_string(),
    };
    let res2 = dispatcher.dispatch(&call2, &mut context).await;
    assert!(
        res2["error"] == "image_already_consumed" || res2["warning"] == "duplicate_call_ignored",
        "Second call must be rejected as duplicate/consumed: {:?}",
        res2
    );
}

#[tokio::test]
async fn test_dispatcher_fuzzy_matching_sesion_2_resolves_to_exact_event() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state.clone());
    let mut setup_context = TurnToolContext::default();

    // 1. Create target event "Shirley Show y sus Amigos - Sesión 2"
    let create_call = ToolCall {
        id: "call_create_s2".to_string(),
        name: "crear_evento".to_string(),
        arguments: json!({
            "nombre": "Shirley Show y sus Amigos - Sesión 2",
            "fecha_hora": "2026-08-15T20:00",
            "lugar": "Teatro Canout",
            "tipos_de_entrada": [
                { "nombre": "General", "precio": 50.0, "capacidad": 100 }
            ]
        })
        .to_string(),
    };
    let create_res = dispatcher.dispatch(&create_call, &mut setup_context).await;
    let expected_event_id = create_res["id"].as_str().unwrap().to_string();

    // 2. Fresh context with NO known events or active event
    let mut fresh_context = TurnToolContext::default();
    assert!(!fresh_context.known_event_ids.contains(&expected_event_id));
    assert_eq!(fresh_context.active_event_id, None);

    // Call vender_entradas with substring "sesión 2"
    let sell_call = ToolCall {
        id: "call_sell_fuzzy_1".to_string(),
        name: "vender_entradas".to_string(),
        arguments: json!({
            "evento_id": "sesión 2",
            "nombre_comprador": "María Quispe",
            "contacto": "987654321",
            "tipo_entrada": "General",
            "cantidad": 1,
            "medio_pago": "Yape",
            "estado_pago": "Pagado"
        })
        .to_string(),
    };

    let sell_res = dispatcher.dispatch(&sell_call, &mut fresh_context).await;
    assert!(
        sell_res.get("entradas").is_some(),
        "Expected successful sale, got: {sell_res}"
    );
    assert_eq!(sell_res["total_entradas"], 1);
    assert!(fresh_context.known_event_ids.contains(&expected_event_id));

    // 3. Substring without tilde "sesion 2" on resumen_evento with fresh context
    let mut fresh_context_2 = TurnToolContext::default();
    let summary_call = ToolCall {
        id: "call_sum_fuzzy_2".to_string(),
        name: "resumen_evento".to_string(),
        arguments: json!({ "evento_id": "sesion 2" }).to_string(),
    };
    let sum_res = dispatcher
        .dispatch(&summary_call, &mut fresh_context_2)
        .await;
    assert_eq!(sum_res["evento_id"], expected_event_id);
    assert_eq!(sum_res["entradas_emitidas"], 1);
    assert_eq!(sum_res["cobrado"], 50.0);
}

#[tokio::test]
async fn test_dispatcher_fuzzy_matching_ambiguous_event_returns_clarification_error() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state.clone());
    let mut setup_context = TurnToolContext::default();

    // 1. Create two events with shared substring
    let create_1 = ToolCall {
        id: "call_create_ambig_1".to_string(),
        name: "crear_evento".to_string(),
        arguments: json!({
            "nombre": "Shirley Show y sus Amigos - Sesión 1",
            "fecha_hora": "2026-08-14T20:00",
            "lugar": "Teatro Canout",
            "tipos_de_entrada": [{ "nombre": "General", "precio": 50.0, "capacidad": 100 }]
        })
        .to_string(),
    };
    dispatcher.dispatch(&create_1, &mut setup_context).await;

    let create_2 = ToolCall {
        id: "call_create_ambig_2".to_string(),
        name: "crear_evento".to_string(),
        arguments: json!({
            "nombre": "Shirley Show y sus Amigos - Sesión 2",
            "fecha_hora": "2026-08-15T20:00",
            "lugar": "Teatro Canout",
            "tipos_de_entrada": [{ "nombre": "General", "precio": 50.0, "capacidad": 100 }]
        })
        .to_string(),
    };
    dispatcher.dispatch(&create_2, &mut setup_context).await;

    // 2. Fresh context calls resumen_evento with ambiguous substring "sesión"
    let mut fresh_context = TurnToolContext::default();
    let ambig_call = ToolCall {
        id: "call_sum_ambig".to_string(),
        name: "resumen_evento".to_string(),
        arguments: json!({ "evento_id": "sesión" }).to_string(),
    };

    let res = dispatcher.dispatch(&ambig_call, &mut fresh_context).await;
    assert_eq!(res["error"], "ambiguous_event_id");
    assert!(
        res["message"]
            .as_str()
            .unwrap()
            .contains("Múltiples eventos")
    );
    let candidatos = res["candidatos"].as_array().expect("candidatos array");
    assert_eq!(candidatos.len(), 2);
    assert!(
        candidatos
            .iter()
            .any(|c| c.as_str().unwrap().contains("Sesión 1"))
    );
    assert!(
        candidatos
            .iter()
            .any(|c| c.as_str().unwrap().contains("Sesión 2"))
    );
}

#[tokio::test]
async fn test_dispatcher_fuzzy_matching_presentation_by_substring_and_ambiguity() {
    let (state, _tmp) = setup_test_state();
    let dispatcher = AgentToolDispatcher::new(state.clone());

    // 1. Create single presentation "Show Acústico - Sesión 2"
    let pres = state
        .create_presentation
        .execute("Show Acústico - Sesión 2")
        .unwrap();

    let mut context = TurnToolContext::default();

    // Query with "sesión 2" must resolve to the presentation
    let get_call = ToolCall {
        id: "call_get_pres_fuzzy".to_string(),
        name: "get_presentation".to_string(),
        arguments: json!({ "presentation_id": "sesión 2" }).to_string(),
    };
    let get_res = dispatcher.dispatch(&get_call, &mut context).await;
    assert_eq!(get_res["id"], pres.id);
    assert_eq!(get_res["title"], "Show Acústico - Sesión 2");
    assert!(context.known_presentation_ids.contains(&pres.id));

    // 2. Create another presentation "Show Acústico - Sesión 1"
    let _pres1 = state
        .create_presentation
        .execute("Show Acústico - Sesión 1")
        .unwrap();

    // Query with "acústico" is now ambiguous (matches both)
    let mut context_ambig = TurnToolContext::default();
    let ambig_call = ToolCall {
        id: "call_get_pres_ambig".to_string(),
        name: "get_presentation".to_string(),
        arguments: json!({ "presentation_id": "acústico" }).to_string(),
    };
    let ambig_res = dispatcher.dispatch(&ambig_call, &mut context_ambig).await;
    assert_eq!(ambig_res["error"], "ambiguous_presentation_id");
    assert!(
        ambig_res["message"]
            .as_str()
            .unwrap()
            .contains("Múltiples presentaciones")
    );
    let candidatos = ambig_res["candidatos"].as_array().unwrap();
    assert_eq!(candidatos.len(), 2);
}
