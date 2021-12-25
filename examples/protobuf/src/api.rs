use std::sync::Arc;

use thaloto::{
    aggregate::Aggregate,
    event::IntoEvents,
    event_store::EventStore,
    tests_cfg::bank_account::{BankAccount, BankAccountCommand},
};
use thaloto_inmemory::InMemoryEventStore;
use tonic::{Request, Status};

tonic::include_proto!("bank_account");

#[derive(Clone, Debug, Default)]
pub struct BankAccountService {
    event_store: Arc<InMemoryEventStore>,
}

#[tonic::async_trait]
impl bank_account_server::BankAccount for BankAccountService {
    async fn open_account(
        &self,
        request: Request<OpenAccountCommand>,
    ) -> Result<tonic::Response<Response>, Status> {
        let command = request.into_inner();

        let exists = self
            .event_store
            .load_aggregate_sequence::<BankAccount>(&command.id)
            .await
            .map_err(|err| Status::internal(err.to_string()))?
            .is_some();
        if exists {
            return Err(Status::already_exists("account already opened"));
        }

        let (bank_account, event) = BankAccount::open_account(command.id, command.initial_balance);
        let events = event.into_events();

        self.event_store
            .save_events::<BankAccount>(bank_account.id(), &events)
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        let events_json = serde_json::to_string(&events)
            .map_err(|_| Status::internal("failed to serialize events"))?;

        Ok(tonic::Response::new(Response {
            events: events_json,
        }))
    }

    async fn deposit_funds(
        &self,
        request: Request<DepositFundsCommand>,
    ) -> Result<tonic::Response<Response>, Status> {
        let command = request.into_inner();

        let bank_account: BankAccount = self
            .event_store
            .load_aggregate(command.id)
            .await
            .map_err(|err| Status::internal(err.to_string()))?
            .ok_or_else(|| Status::not_found("account does not exist"))?;

        let events = bank_account.deposit_funds(command.amount)?.into_events();

        self.event_store
            .save_events::<BankAccount>(bank_account.id(), &events)
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        let events_json = serde_json::to_string(&events)
            .map_err(|_| Status::internal("failed to serialize events"))?;

        Ok(tonic::Response::new(Response {
            events: events_json,
        }))
    }

    async fn withdraw_funds(
        &self,
        request: Request<WithdrawFundsCommand>,
    ) -> Result<tonic::Response<Response>, Status> {
        let command = request.into_inner();

        let bank_account: BankAccount = self
            .event_store
            .load_aggregate(command.id)
            .await
            .map_err(|err| Status::internal(err.to_string()))?
            .ok_or_else(|| Status::not_found("account does not exist"))?;

        let events = bank_account.withdraw_funds(command.amount)?.into_events();

        self.event_store
            .save_events::<BankAccount>(bank_account.id(), &events)
            .await
            .map_err(|err| Status::internal(err.to_string()))?;

        let events_json = serde_json::to_string(&events)
            .map_err(|_| Status::internal("failed to serialize events"))?;

        Ok(tonic::Response::new(Response {
            events: events_json,
        }))
    }
}
