use async_trait::async_trait;
use awto_es::{Error, InternalError, Repository};
use bb8_postgres::{
    bb8::{Pool, PooledConnection},
    tokio_postgres::{
        tls::{MakeTlsConnect, TlsConnect},
        Socket,
    },
    PostgresConnectionManager,
};

pub struct BankAccountView {
    pub account_number: String,
    pub balance: f64,
}

impl BankAccountView {
    pub fn new(account_number: String, balance: f64) -> Self {
        Self {
            account_number,
            balance,
        }
    }
}

pub struct BankAccountViewRepository<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    pool: Pool<PostgresConnectionManager<Tls>>,
}

impl<Tls> BankAccountViewRepository<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    pub async fn connect(
        conn: &str,
        tls: Tls,
    ) -> Result<Self, bb8_postgres::tokio_postgres::Error> {
        let manager = PostgresConnectionManager::new_from_stringlike(conn, tls)?;
        let pool = Pool::builder().build(manager).await.unwrap();

        Ok(Self { pool })
    }

    async fn get_conn(
        &self,
    ) -> Result<PooledConnection<'_, PostgresConnectionManager<Tls>>, Error> {
        self.pool
            .get()
            .await
            .internal_error("could not get connection from pool")
    }
}

#[async_trait]
impl<Tls> Repository<BankAccountView, String> for BankAccountViewRepository<Tls>
where
    Tls: MakeTlsConnect<Socket> + Clone + Send + Sync + 'static,
    <Tls as MakeTlsConnect<Socket>>::Stream: Send + Sync,
    <Tls as MakeTlsConnect<Socket>>::TlsConnect: Send,
    <<Tls as MakeTlsConnect<Socket>>::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    /// Insert or update view
    async fn save(&self, view: &BankAccountView, event_sequence: i64) -> Result<(), Error> {
        let conn = self.get_conn().await?;

        conn.execute(
            "
            INSERT INTO bank_account (account_number, last_event_sequence, balance) 
            VALUES ($1, $2, $3)
            ON CONFLICT (account_number) DO UPDATE 
            SET last_event_sequence = $2, balance = $3;
            ",
            &[&view.account_number, &event_sequence, &view.balance],
        )
        .await
        .internal_error("could not insert/update bank account view")?;

        Ok(())
    }

    /// Load an existing view
    async fn load(&self, id: &String) -> Result<Option<(BankAccountView, i64)>, Error> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_opt(
                "
                SELECT last_event_sequence, balance FROM bank_account WHERE account_number = $1;
                ",
                &[id],
            )
            .await
            .internal_error("could not select bank account view")?;

        Ok(row.map(|row| (BankAccountView::new(id.clone(), row.get(1)), row.get(0))))
    }

    /// Delete an existing view
    async fn delete(&self, id: &String) -> Result<(), Error> {
        let conn = self.get_conn().await?;

        conn.execute("DELETE FROM bank_account WHERE account_number = $1", &[id])
            .await
            .internal_error("could not insert/update bank account view")?;

        Ok(())
    }
}
