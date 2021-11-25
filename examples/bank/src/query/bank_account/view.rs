use awto_es::PgRepository;

#[derive(PgRepository)]
pub struct BankAccountView {
    pub account_number: String,
    pub balance: f64,
}
