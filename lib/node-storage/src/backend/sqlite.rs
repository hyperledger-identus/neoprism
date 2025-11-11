use std::marker::PhantomData;

use lazybe::db::sqlite::SqliteDbCtx;

use crate::Error;

#[derive(Debug, Clone)]
pub struct SqliteDb {
    _ctx: PhantomData<SqliteDbCtx>,
}

impl SqliteDb {
    pub async fn connect(_db_url: &str) -> Result<Self, Error> {
        todo!("SqliteDb::connect is not implemented yet");
    }

    pub async fn migrate(&self) -> Result<(), Error> {
        todo!("SqliteDb::migrate is not implemented yet");
    }
}
