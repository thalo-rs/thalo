use api::bank_account_server::BankAccountServer;
use api::BankAccountService;
use text_io::read;
use tonic::transport::Server;

mod api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();

    let bank_account_service = BankAccountService::default();

    println!("BankAccountService listening on {}", addr);
    println!("Press any key to print event store");

    {
        let bank_account_service = bank_account_service.clone();
        tokio::task::spawn_blocking(move || loop {
            let _: String = read!();
            println!("Event Store:\n{:#?}", bank_account_service);
        });
    }

    Server::builder()
        .add_service(BankAccountServer::new(bank_account_service))
        .serve(addr)
        .await?;

    Ok(())
}
