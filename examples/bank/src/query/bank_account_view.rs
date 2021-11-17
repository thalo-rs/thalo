use async_trait::async_trait;
use awto_es::{postgres::tls::NoTls, Error, InternalError, Repository};
use bb8_postgres::{
    bb8::{Pool, PooledConnection},
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

#[derive(Clone)]
pub struct BankAccountViewRepository {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

impl BankAccountViewRepository {
    pub async fn connect(
        conn: &str,
        tls: NoTls,
    ) -> Result<Self, bb8_postgres::tokio_postgres::Error> {
        let manager = PostgresConnectionManager::new_from_stringlike(conn, tls)?;
        let pool = Pool::builder().build(manager).await.unwrap();

        Ok(Self { pool })
    }

    async fn get_conn(
        &self,
    ) -> Result<PooledConnection<'_, PostgresConnectionManager<NoTls>>, Error> {
        self.pool
            .get()
            .await
            .internal_error("could not get connection from pool")
    }
}

#[async_trait]
impl Repository<BankAccountView> for BankAccountViewRepository {
    /// Insert or update view
    async fn save(
        &self,
        view: &BankAccountView,
        event_id: i64,
        event_sequence: i64,
    ) -> Result<(), Error> {
        let conn = self.get_conn().await?;

        conn.execute(
            "
            INSERT INTO bank_account (account_number, last_event_id, last_event_sequence, balance) 
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (account_number) DO UPDATE 
            SET last_event_id = $2, last_event_sequence = $3, balance = $4;
            ",
            &[
                &view.account_number,
                &event_id,
                &event_sequence,
                &view.balance,
            ],
        )
        .await
        .internal_error("could not insert/update bank account view")?;

        Ok(())
    }

    /// Load an existing view
    async fn load(&self, id: &str) -> Result<Option<(BankAccountView, i64)>, Error> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_opt(
                "
                SELECT last_event_sequence, balance FROM bank_account WHERE account_number = $1;
                ",
                &[&id],
            )
            .await
            .internal_error("could not select bank account view")?;

        Ok(row.map(|row| (BankAccountView::new(id.to_string(), row.get(1)), row.get(0))))
    }

    /// Delete an existing view
    async fn delete(&self, id: &str) -> Result<(), Error> {
        let conn = self.get_conn().await?;

        conn.execute("DELETE FROM bank_account WHERE account_number = $1", &[&id])
            .await
            .internal_error("could not insert/update bank account view")?;

        Ok(())
    }

    /// Load the latest event version number
    async fn last_event_id(&self) -> Result<Option<i64>, Error> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(r#"SELECT MAX("last_event_id") FROM "bank_account""#, &[])
            .await
            .internal_error("could not query max last_event_id")?;

        Ok(row.get(0))
    }

    /// Load the latest event version number
    async fn last_event_sequence(&self, id: &str) -> Result<Option<i64>, Error> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_opt(
                r#"SELECT "last_event_sequence" FROM "bank_account" WHERE "id" = $1"#,
                &[&id],
            )
            .await
            .internal_error("could not query max last_event_id")?;

        Ok(row.map(|row| row.get(0)))
    }
}
