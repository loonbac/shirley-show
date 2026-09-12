use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;

use shirley_show::agent::tool_dispatcher::AgentToolDispatcher;
use shirley_show::agent::tool_schemas::get_tool_definitions;
use shirley_show::agent::turn_context::TurnToolContext;
use shirley_show::domain::agent_tool::ToolCall;
use shirley_show::infra::filesystem::LocalAudioStorage;
use shirley_show::infra::sqlite::{init_pool, migrate_pool};
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
        16,
        "get_tool_definitions() must expose exactly 16 tools (8 presentations + 8 ticketing)"
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

    // 8 ticketing tools
    assert!(names.contains(&"crear_evento"));
    assert!(names.contains(&"listar_eventos"));
    assert!(names.contains(&"vender_entradas"));
    assert!(names.contains(&"confirmar_pago_entrada"));
    assert!(names.contains(&"registrar_gasto"));
    assert!(names.contains(&"resumen_evento"));
    assert!(names.contains(&"registrar_interesado"));
    assert!(names.contains(&"checkin_entrada"));

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
