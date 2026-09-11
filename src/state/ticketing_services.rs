use std::sync::Arc;

use crate::infra::random_credentials::OsRandomCredentials;
use crate::infra::sqlite::ticketing_repository::SqliteTicketingRepository;
use crate::use_cases::ticketing::checkin::CheckInTicket;
use crate::use_cases::ticketing::events::{
    CreateEvent, DeleteEvent, FinalizeEvent, GetEvent, ListEvents, ReopenEvent, UpdateEvent,
};
use crate::use_cases::ticketing::expenses::{
    CreateExpense, DeleteExpense, GetExpense, ListExpenses, UpdateExpense,
};
use crate::use_cases::ticketing::summary::GetEventSummary;
use crate::use_cases::ticketing::tickets::{
    ConfirmTicketPayment, DeleteTicket, GetPublicTicket, GetTicket, ListTickets, SellTickets,
    UpdateTicketContact,
};

#[derive(Clone)]
pub struct TicketingServices {
    pub repo: Arc<SqliteTicketingRepository>,
    pub create_event: Arc<CreateEvent<SqliteTicketingRepository>>,
    pub list_events: Arc<ListEvents<SqliteTicketingRepository>>,
    pub get_event: Arc<GetEvent<SqliteTicketingRepository>>,
    pub update_event: Arc<UpdateEvent<SqliteTicketingRepository>>,
    pub finalize_event: Arc<FinalizeEvent<SqliteTicketingRepository>>,
    pub reopen_event: Arc<ReopenEvent<SqliteTicketingRepository>>,
    pub delete_event: Arc<DeleteEvent<SqliteTicketingRepository>>,
    pub create_expense: Arc<CreateExpense<SqliteTicketingRepository>>,
    pub list_expenses: Arc<ListExpenses<SqliteTicketingRepository>>,
    pub get_expense: Arc<GetExpense<SqliteTicketingRepository>>,
    pub update_expense: Arc<UpdateExpense<SqliteTicketingRepository>>,
    pub delete_expense: Arc<DeleteExpense<SqliteTicketingRepository>>,
    pub get_summary: Arc<GetEventSummary<SqliteTicketingRepository>>,
    pub sell_tickets: Arc<
        SellTickets<
            SqliteTicketingRepository,
            SqliteTicketingRepository,
            OsRandomCredentials,
        >,
    >,
    pub list_tickets: Arc<ListTickets<SqliteTicketingRepository>>,
    pub get_ticket: Arc<GetTicket<SqliteTicketingRepository>>,
    pub update_ticket_contact: Arc<UpdateTicketContact<SqliteTicketingRepository>>,
    pub delete_ticket: Arc<DeleteTicket<SqliteTicketingRepository>>,
    pub confirm_ticket_payment: Arc<ConfirmTicketPayment<SqliteTicketingRepository>>,
    pub get_public_ticket: Arc<
        GetPublicTicket<
            SqliteTicketingRepository,
            SqliteTicketingRepository,
        >,
    >,
    pub check_in_ticket: Arc<CheckInTicket<SqliteTicketingRepository>>,
}

impl TicketingServices {
    pub fn new(repo: Arc<SqliteTicketingRepository>) -> Self {
        let creds_gen = Arc::new(OsRandomCredentials::new());
        Self {
            create_event: Arc::new(CreateEvent::new(repo.clone())),
            list_events: Arc::new(ListEvents::new(repo.clone())),
            get_event: Arc::new(GetEvent::new(repo.clone())),
            update_event: Arc::new(UpdateEvent::new(repo.clone())),
            finalize_event: Arc::new(FinalizeEvent::new(repo.clone())),
            reopen_event: Arc::new(ReopenEvent::new(repo.clone())),
            delete_event: Arc::new(DeleteEvent::new(repo.clone())),
            create_expense: Arc::new(CreateExpense::new(repo.clone())),
            list_expenses: Arc::new(ListExpenses::new(repo.clone())),
            get_expense: Arc::new(GetExpense::new(repo.clone())),
            update_expense: Arc::new(UpdateExpense::new(repo.clone())),
            delete_expense: Arc::new(DeleteExpense::new(repo.clone())),
            get_summary: Arc::new(GetEventSummary::new(repo.clone())),
            sell_tickets: Arc::new(SellTickets::new(repo.clone(), repo.clone(), creds_gen)),
            list_tickets: Arc::new(ListTickets::new(repo.clone())),
            get_ticket: Arc::new(GetTicket::new(repo.clone())),
            update_ticket_contact: Arc::new(UpdateTicketContact::new(repo.clone())),
            delete_ticket: Arc::new(DeleteTicket::new(repo.clone())),
            confirm_ticket_payment: Arc::new(ConfirmTicketPayment::new(repo.clone())),
            get_public_ticket: Arc::new(GetPublicTicket::new(repo.clone(), repo.clone())),
            check_in_ticket: Arc::new(CheckInTicket::new(repo.clone())),
            repo,
        }
    }
}
