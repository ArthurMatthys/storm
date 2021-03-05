mod client_factory;
mod execute;
mod filter_sql;
mod from_sql;
mod mssql_provider;
mod parameter;
mod query_rows;
mod save_entity_part;
mod upsert_builder;

pub use client_factory::ClientFactory;
pub use execute::Execute;
pub use filter_sql::FilterSql;
pub use from_sql::FromSql;
pub use mssql_provider::{MssqlProvider, MssqlTransaction};
pub use parameter::Parameter;
pub use query_rows::QueryRows;
pub use save_entity_part::SaveEntityPart;
pub use storm::{Error, Result};
pub use tiberius;
pub use upsert_builder::UpsertBuilder;

pub type Client = tiberius::Client<tokio_util::compat::Compat<tokio::net::TcpStream>>;

#[async_trait::async_trait]
pub trait MyTrait {
    async fn test(&self) -> storm::Result<()>;
}

struct Provider;

impl Provider {
    async fn provide(&self) -> storm::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl MyTrait for Provider {
    async fn test(&self) -> storm::Result<()> {
        self.provide()
            .await
            .map_err(|_| storm::Error::ClientInError)?;
        Ok(())
    }
}
