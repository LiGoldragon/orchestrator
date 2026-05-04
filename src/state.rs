//! redb-backed cursor and dispatch state.

use std::fs;
use std::path::{Path, PathBuf};

use redb::{Database, ReadableTable, TableDefinition};

use crate::{CascadeDispatchRecord, Error, EventSequence, Result};

const CURSOR_TABLE: TableDefinition<&str, u64> = TableDefinition::new("cursor");
const DISPATCH_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("dispatch");
const CURSOR_KEY: &str = "event-sequence";

pub struct EventCursor {
    database: Database,
    path: PathBuf,
}

impl EventCursor {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let database = if path.exists() {
            Database::open(&path).map_err(Error::state)?
        } else {
            Database::create(&path).map_err(Error::state)?
        };
        let cursor = Self { database, path };
        cursor.ensure_tables()?;
        Ok(cursor)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn current(&self) -> Result<Option<EventSequence>> {
        let read_transaction = self.database.begin_read().map_err(Error::state)?;
        let table = read_transaction
            .open_table(CURSOR_TABLE)
            .map_err(Error::state)?;
        table
            .get(CURSOR_KEY)
            .map(|cursor| cursor.map(|sequence| EventSequence::new(sequence.value())))
            .map_err(Error::state)
    }

    pub fn advance(&self, sequence: EventSequence) -> Result<()> {
        let write_transaction = self.database.begin_write().map_err(Error::state)?;
        {
            let mut table = write_transaction
                .open_table(CURSOR_TABLE)
                .map_err(Error::state)?;
            table
                .insert(CURSOR_KEY, &sequence.value())
                .map_err(Error::state)?;
        }
        write_transaction.commit().map_err(Error::state)
    }

    pub fn record_dispatch(&self, record: &CascadeDispatchRecord) -> Result<()> {
        let bytes = record.archived_bytes()?;
        let write_transaction = self.database.begin_write().map_err(Error::state)?;
        {
            let mut table = write_transaction
                .open_table(DISPATCH_TABLE)
                .map_err(Error::state)?;
            table
                .insert(&record.event_sequence(), bytes.as_slice())
                .map_err(Error::state)?;
        }
        write_transaction.commit().map_err(Error::state)
    }

    pub fn recorded_dispatch_count(&self) -> Result<usize> {
        let read_transaction = self.database.begin_read().map_err(Error::state)?;
        let table = read_transaction
            .open_table(DISPATCH_TABLE)
            .map_err(Error::state)?;
        let mut count = 0;
        for row in table.iter().map_err(Error::state)? {
            row.map_err(Error::state)?;
            count += 1;
        }
        Ok(count)
    }

    fn ensure_tables(&self) -> Result<()> {
        let write_transaction = self.database.begin_write().map_err(Error::state)?;
        {
            write_transaction
                .open_table(CURSOR_TABLE)
                .map_err(Error::state)?;
            write_transaction
                .open_table(DISPATCH_TABLE)
                .map_err(Error::state)?;
        }
        write_transaction.commit().map_err(Error::state)
    }
}
