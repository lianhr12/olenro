//! Database module
//!
//! Provides database access using rusqlite

pub mod dao;

use crate::error::{AppError, AppResult};
use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::Mutex;

/// Database wrapper
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create database at the given path
    pub fn open(path: &PathBuf) -> AppResult<Self> {
        let conn = Connection::open(path).map_err(|e| AppError::Database(e))?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| AppError::Database(e))?;

        Ok(Self { conn })
    }

    /// Get the underlying connection (for advanced use)
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Initialize database schema
    pub fn init_schema(&self) -> AppResult<()> {
        self.conn
            .execute_batch(include_str!("schema.sql"))
            .map_err(|e| AppError::Database(e))?;
        Ok(())
    }
}

/// Thread-safe database wrapper
pub type SharedDatabase = Arc<Mutex<Database>>;

pub use dao::*;

use std::sync::Arc;
