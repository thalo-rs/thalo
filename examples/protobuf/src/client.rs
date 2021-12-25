use std::io::{stdout, Write};
use std::{fmt, io, str};

use api::bank_account_client::BankAccountClient;
use api::{DepositFundsCommand, OpenAccountCommand, WithdrawFundsCommand};
use crossterm::{cursor, execute};
use text_io::{read, try_read};
use tonic::{transport::Channel, Status};

mod api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = BankAccountClient::connect("http://[::1]:50051").await?;

    print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

    loop {
        println!("Choose a command:");
        println!("  1) Open an account");
        println!("  2) Deposit funds");
        println!("  3) Withdraw funds");
        execute!(stdout(), cursor::SavePosition).ok();
        let input: String = read("");
        let response = match input.as_str() {
            "1" => {
                execute!(stdout(), cursor::RestorePosition).ok();
                println!("Open an account");
                open_account(&mut client).await
            }
            "2" => {
                execute!(stdout(), cursor::RestorePosition).ok();
                println!("Deposit funds");
                deposit_funds(&mut client).await
            }
            "3" => {
                execute!(stdout(), cursor::RestorePosition).ok();
                println!("Deposit funds");
                withdraw_funds(&mut client).await
            }
            _ => {
                println!("[error] invalid input\n");
                continue;
            }
        };

        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

        match response {
            Ok(_) => println!("[ok]"),
            Err(status) => println!("Command failed: {}\n", status.message()),
        }
    }
}

async fn open_account(
    client: &mut BankAccountClient<Channel>,
) -> Result<tonic::Response<api::Response>, Status> {
    let id: String = read("Enter account ID");
    let initial_balance: f64 = try_read("Enter initial balance");

    let command = tonic::Request::new(OpenAccountCommand {
        id,
        initial_balance,
    });

    client.open_account(command).await
}

async fn deposit_funds(
    client: &mut BankAccountClient<Channel>,
) -> Result<tonic::Response<api::Response>, Status> {
    let id: String = read("Enter account ID");
    let amount: f64 = try_read("Enter deposit amount");

    let command = tonic::Request::new(DepositFundsCommand { id, amount });

    client.deposit_funds(command).await
}

async fn withdraw_funds(
    client: &mut BankAccountClient<Channel>,
) -> Result<tonic::Response<api::Response>, Status> {
    let id: String = read("Enter account ID");
    let amount: f64 = try_read("Enter withdraw amount");

    let command = tonic::Request::new(WithdrawFundsCommand { id, amount });

    client.withdraw_funds(command).await
}

fn read<T>(msg: &str) -> T
where
    T: fmt::Display + str::FromStr,
    <T as std::str::FromStr>::Err: fmt::Debug,
{
    if !msg.is_empty() {
        print!("{}: ", msg);
    }
    io::stdout().flush().unwrap();
    read!()
}

fn try_read<T>(msg: impl fmt::Display) -> T
where
    T: fmt::Display + str::FromStr,
    <T as std::str::FromStr>::Err: fmt::Debug,
{
    loop {
        print!("{}: ", msg);
        io::stdout().flush().unwrap();
        match try_read!() {
            Ok(val) => return val,
            Err(_) => println!("Invalid input"),
        }
    }
}
