use std::sync::Mutex;

use shirley_show::domain::Presentation;
use shirley_show::error::AppError;
use shirley_show::ports::presentation_reader::PresentationReader;
use shirley_show::ports::presentation_writer::{DeleteResult, PresentationWriter};
use shirley_show::use_cases::create_presentation::CreatePresentation;
use shirley_show::use_cases::delete_presentation::DeletePresentation;
use shirley_show::use_cases::get_presentation::GetPresentation;
use shirley_show::use_cases::list_presentations::ListPresentations;
use shirley_show::use_cases::rename_presentation::RenamePresentation;

#[derive(Default)]
struct MockPresentationPort {
    items: Mutex<Vec<Presentation>>,
}

impl PresentationReader for MockPresentationPort {
    fn list(&self) -> Result<Vec<Presentation>, AppError> {
        Ok(self.items.lock().unwrap().clone())
    }

    fn get(&self, id: &str) -> Result<Option<Presentation>, AppError> {
        let items = self.items.lock().unwrap();
        Ok(items.iter().find(|p| p.id == id).cloned())
    }
}

impl PresentationWriter for MockPresentationPort {
    fn create(&self, presentation: &Presentation) -> Result<(), AppError> {
        self.items.lock().unwrap().push(presentation.clone());
        Ok(())
    }

    fn rename(&self, id: &str, title: &str, updated_at: &str) -> Result<Presentation, AppError> {
        let mut items = self.items.lock().unwrap();
        if let Some(p) = items.iter_mut().find(|p| p.id == id) {
            p.title = title.to_string();
            p.updated_at = updated_at.to_string();
            Ok(p.clone())
        } else {
            Err(AppError::not_found(
                "presentation_not_found",
                "Presentación no encontrada.",
            ))
        }
    }

    fn delete(&self, id: &str) -> Result<DeleteResult, AppError> {
        let mut items = self.items.lock().unwrap();
        if let Some(idx) = items.iter().position(|p| p.id == id) {
            items.remove(idx);
            Ok(DeleteResult {
                deleted_id: id.to_string(),
                cleanup_pending: false,
            })
        } else {
            Err(AppError::not_found(
                "presentation_not_found",
                "Presentación no encontrada.",
            ))
        }
    }
}

#[test]
fn test_create_presentation_use_case() {
    let port = std::sync::Arc::new(MockPresentationPort::default());
    let use_case = CreatePresentation::new(port);

    let created = use_case.execute("  Viernes Acústico  ").expect("success");
    assert_eq!(created.title, "Viernes Acústico");

    let err_empty = use_case.execute("").unwrap_err();
    match err_empty {
        AppError::Validation { code, field, .. } => {
            assert_eq!(code, "title_required");
            assert_eq!(field.as_deref(), Some("title"));
        }
        other => panic!("expected AppError::Validation, got {:?}", other),
    }

    let err_whitespace = use_case.execute("   \n\t ").unwrap_err();
    match err_whitespace {
        AppError::Validation { code, field, .. } => {
            assert_eq!(code, "title_required");
            assert_eq!(field.as_deref(), Some("title"));
        }
        other => panic!("expected AppError::Validation, got {:?}", other),
    }
}

#[test]
fn test_list_presentations_use_case() {
    let port = std::sync::Arc::new(MockPresentationPort::default());
    let create_uc = CreatePresentation::new(port.clone());
    let list_uc = ListPresentations::new(port.clone());

    assert_eq!(list_uc.execute().unwrap().len(), 0);

    create_uc.execute("Show 1").unwrap();
    create_uc.execute("Show 2").unwrap();

    let list = list_uc.execute().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].title, "Show 1");
    assert_eq!(list[1].title, "Show 2");
}

#[test]
fn test_get_presentation_use_case() {
    let port = std::sync::Arc::new(MockPresentationPort::default());
    let create_uc = CreatePresentation::new(port.clone());
    let get_uc = GetPresentation::new(port.clone());

    let created = create_uc.execute("Show 1").unwrap();

    let fetched = get_uc.execute(&created.id).unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.title, "Show 1");

    let err = get_uc.execute("missing-id").unwrap_err();
    match err {
        AppError::NotFound { code, .. } => {
            assert_eq!(code, "presentation_not_found");
        }
        other => panic!("expected AppError::NotFound, got {:?}", other),
    }
}

#[test]
fn test_rename_presentation_use_case() {
    let port = std::sync::Arc::new(MockPresentationPort::default());
    let create_uc = CreatePresentation::new(port.clone());
    let rename_uc = RenamePresentation::new(port.clone());

    let created = create_uc.execute("Original").unwrap();

    let renamed = rename_uc.execute(&created.id, "  Renamed  ").unwrap();
    assert_eq!(renamed.title, "Renamed");
    assert_eq!(renamed.id, created.id);

    let err_empty = rename_uc.execute(&created.id, "   ").unwrap_err();
    match err_empty {
        AppError::Validation { code, field, .. } => {
            assert_eq!(code, "title_required");
            assert_eq!(field.as_deref(), Some("title"));
        }
        other => panic!("expected AppError::Validation, got {:?}", other),
    }

    let err_missing = rename_uc.execute("missing-id", "New").unwrap_err();
    match err_missing {
        AppError::NotFound { code, .. } => {
            assert_eq!(code, "presentation_not_found");
        }
        other => panic!("expected AppError::NotFound, got {:?}", other),
    }
}

#[test]
fn test_delete_presentation_use_case() {
    let port = std::sync::Arc::new(MockPresentationPort::default());
    let create_uc = CreatePresentation::new(port.clone());
    let delete_uc = DeletePresentation::new(port.clone());

    let created = create_uc.execute("To Delete").unwrap();

    let res = delete_uc.execute(&created.id).unwrap();
    assert_eq!(res.deleted_id, created.id);
    assert!(!res.cleanup_pending);

    let err_missing = delete_uc.execute(&created.id).unwrap_err();
    match err_missing {
        AppError::NotFound { code, .. } => {
            assert_eq!(code, "presentation_not_found");
        }
        other => panic!("expected AppError::NotFound, got {:?}", other),
    }
}
