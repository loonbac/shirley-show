use shirley_show::domain::Presentation;
use shirley_show::error::AppError;

#[test]
fn test_create_presentation_trims_whitespace() {
    let presentation = Presentation::new("  Viernes 24 Acústico  ").expect("valid title");
    assert_eq!(presentation.title, "Viernes 24 Acústico");
    assert!(!presentation.id.is_empty());
    assert!(!presentation.created_at.is_empty());
    assert_eq!(presentation.created_at, presentation.updated_at);
}

#[test]
fn test_create_presentation_rejects_empty_or_whitespace_title() {
    let err_empty = Presentation::new("").unwrap_err();
    assert!(matches!(err_empty, AppError::Validation { .. }));

    let err_whitespace = Presentation::new("   \t\n  ").unwrap_err();
    assert!(matches!(err_whitespace, AppError::Validation { .. }));
}

#[test]
fn test_create_presentation_unique_ids_for_identical_titles() {
    let p1 = Presentation::new("Ensayo General").expect("p1");
    let p2 = Presentation::new("Ensayo General").expect("p2");
    assert_eq!(p1.title, p2.title);
    assert_ne!(p1.id, p2.id);
}

#[test]
fn test_rename_presentation_success() {
    let mut presentation = Presentation::new("Setlist Borrador").expect("valid title");
    let old_id = presentation.id.clone();
    let old_created = presentation.created_at.clone();

    presentation
        .rename("  Setlist Final  ")
        .expect("valid rename");
    assert_eq!(presentation.id, old_id);
    assert_eq!(presentation.created_at, old_created);
    assert_eq!(presentation.title, "Setlist Final");
}

#[test]
fn test_rename_presentation_rejects_empty_or_whitespace() {
    let mut presentation = Presentation::new("Show Acústico").expect("valid title");
    let err = presentation.rename("   ").unwrap_err();
    assert!(matches!(err, AppError::Validation { .. }));
    assert_eq!(presentation.title, "Show Acústico");
}

#[test]
fn test_unicode_and_special_characters_handling() {
    let unicode_title = "  Concierto En Vivo — Ñandú & Güiro 🎸 ✨  ";
    let presentation = Presentation::new(unicode_title).expect("valid unicode title");
    assert_eq!(
        presentation.title,
        "Concierto En Vivo — Ñandú & Güiro 🎸 ✨"
    );

    let mut p = presentation;
    p.rename("   东京演出 2026   ").expect("cjk title");
    assert_eq!(p.title, "东京演出 2026");
}

#[test]
fn test_internal_whitespace_preserved() {
    let title = "  Parte   1  -   Setlist   Acústico  ";
    let p = Presentation::new(title).expect("title");
    assert_eq!(p.title, "Parte   1  -   Setlist   Acústico");
}
