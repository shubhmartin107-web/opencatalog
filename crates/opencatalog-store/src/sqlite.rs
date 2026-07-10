use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use opencatalog_core::error::{CatalogError, CatalogResult};
use opencatalog_core::traits::{CatalogStore, LineageDirection};
use opencatalog_core::types::*;
use parking_lot::Mutex;
use rusqlite::Connection;
use uuid::Uuid;

pub struct SqliteCatalogStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteCatalogStore {
    pub fn new(path: &str) -> CatalogResult<Self> {
        let conn = Connection::open(path)
            .map_err(|e| CatalogError::StorageError(format!("SQLite open: {e}")))?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_tables()?;
        Ok(store)
    }

    pub fn in_memory() -> CatalogResult<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| CatalogError::StorageError(format!("SQLite open: {e}")))?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_tables()?;
        Ok(store)
    }

    fn init_tables(&self) -> CatalogResult<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS datasources (
                id TEXT PRIMARY KEY, name TEXT UNIQUE NOT NULL,
                source_type TEXT NOT NULL, description TEXT,
                connection_config TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS datasets (
                id TEXT PRIMARY KEY, data_source_id TEXT NOT NULL,
                name TEXT UNIQUE NOT NULL, physical_name TEXT NOT NULL,
                dataset_type TEXT NOT NULL, description TEXT,
                tags TEXT NOT NULL DEFAULT '[]', classification TEXT,
                location TEXT, row_count INTEGER,
                last_crawled_at TEXT, created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                metadata TEXT NOT NULL DEFAULT '{}',
                FOREIGN KEY (data_source_id) REFERENCES datasources(id)
            );
            CREATE TABLE IF NOT EXISTS columns (
                id TEXT PRIMARY KEY, dataset_id TEXT NOT NULL,
                name TEXT NOT NULL, column_type TEXT NOT NULL,
                description TEXT, is_nullable INTEGER NOT NULL DEFAULT 1,
                is_primary_key INTEGER NOT NULL DEFAULT 0,
                is_foreign_key INTEGER NOT NULL DEFAULT 0,
                ordinal_position INTEGER NOT NULL,
                classification TEXT, tags TEXT NOT NULL DEFAULT '[]',
                glossary_term_id TEXT,
                metadata TEXT NOT NULL DEFAULT '{}',
                FOREIGN KEY (dataset_id) REFERENCES datasets(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS glossary_terms (
                id TEXT PRIMARY KEY, name TEXT UNIQUE NOT NULL,
                description TEXT NOT NULL, short_description TEXT,
                domain TEXT, synonyms TEXT NOT NULL DEFAULT '[]',
                related_term_ids TEXT NOT NULL DEFAULT '[]',
                custom_properties TEXT NOT NULL DEFAULT '{}',
                status TEXT NOT NULL DEFAULT 'draft',
                created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS term_mappings (
                id TEXT PRIMARY KEY, term_id TEXT NOT NULL,
                dataset_id TEXT NOT NULL, column_id TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (term_id) REFERENCES glossary_terms(id)
            );
            CREATE TABLE IF NOT EXISTS policies (
                id TEXT PRIMARY KEY, name TEXT UNIQUE NOT NULL,
                description TEXT, policy_type TEXT NOT NULL,
                rules TEXT NOT NULL DEFAULT '[]',
                enabled INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 100,
                created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS crawl_runs (
                id TEXT PRIMARY KEY, data_source_id TEXT NOT NULL,
                status TEXT NOT NULL, started_at TEXT NOT NULL,
                completed_at TEXT, datasets_found INTEGER NOT NULL DEFAULT 0,
                events_processed INTEGER NOT NULL DEFAULT 0, error TEXT
            );
            CREATE TABLE IF NOT EXISTS openlineage_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_json TEXT NOT NULL, ingested_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                dataset_id TEXT NOT NULL,
                column_id TEXT,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                UNIQUE(dataset_id, key)
            );
            CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                method TEXT NOT NULL,
                path TEXT NOT NULL,
                status_code INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                user TEXT,
                api_key TEXT
            );
            CREATE TABLE IF NOT EXISTS schema_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                dataset_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                schema_json TEXT NOT NULL,
                diff_from_previous TEXT,
                created_at TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| CatalogError::StorageError(format!("SQLite init: {e}")))?;
        Ok(())
    }
}

fn json_string<T: serde::Serialize>(val: &T) -> String {
    serde_json::to_string(val).unwrap_or_default()
}
fn json_vec<T: serde::de::DeserializeOwned>(val: &str) -> Vec<T> {
    serde_json::from_str(val).unwrap_or_default()
}
fn json_map(val: &str) -> HashMap<String, String> {
    serde_json::from_str(val).unwrap_or_default()
}

fn uuid(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap_or_else(|_| Uuid::nil())
}
fn dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn compute_schema_diff(from: &[Column], to: &[Column]) -> String {
    let mut lines = Vec::new();

    let from_map: HashMap<&str, (&str, bool, bool, bool, Option<&str>)> = from
        .iter()
        .map(|c| {
            (
                c.name.as_str(),
                (
                    c.column_type.as_str(),
                    c.is_nullable,
                    c.is_primary_key,
                    c.is_foreign_key,
                    c.description.as_deref(),
                ),
            )
        })
        .collect();
    let to_map: HashMap<&str, (&str, bool, bool, bool, Option<&str>)> = to
        .iter()
        .map(|c| {
            (
                c.name.as_str(),
                (
                    c.column_type.as_str(),
                    c.is_nullable,
                    c.is_primary_key,
                    c.is_foreign_key,
                    c.description.as_deref(),
                ),
            )
        })
        .collect();

    for col in to {
        if !from_map.contains_key(col.name.as_str()) {
            lines.push(format!("+ {} {} (added)", col.name, col.column_type));
        }
    }

    for col in from {
        if !to_map.contains_key(col.name.as_str()) {
            lines.push(format!("- {} {} (removed)", col.name, col.column_type));
        }
    }

    for col in to {
        if let Some(&(old_type, old_null, old_pk, old_fk, _old_desc)) =
            from_map.get(col.name.as_str())
        {
            let mut changed = false;
            let mut detail = String::new();

            if col.column_type != old_type {
                changed = true;
                detail.push_str(&format!("type: {} -> {}", old_type, col.column_type));
            }
            if col.is_nullable != old_null {
                changed = true;
                detail.push_str(&format!(", nullable: {} -> {}", old_null, col.is_nullable));
            }
            if col.is_primary_key != old_pk {
                changed = true;
                detail.push_str(&format!(
                    ", primary_key: {} -> {}",
                    old_pk, col.is_primary_key
                ));
            }
            if col.is_foreign_key != old_fk {
                changed = true;
                detail.push_str(&format!(
                    ", foreign_key: {} -> {}",
                    old_fk, col.is_foreign_key
                ));
            }
            if changed {
                lines.push(format!("~ {} {} ({})", col.name, col.column_type, detail));
            }
        }
    }

    if lines.is_empty() {
        "no changes".to_string()
    } else {
        lines.join("\n")
    }
}

impl SqliteCatalogStore {
    fn row_to_datasource(&self, row: &rusqlite::Row) -> rusqlite::Result<DataSource> {
        Ok(DataSource {
            id: uuid(&row.get::<_, String>(0)?),
            name: row.get(1)?,
            source_type: serde_json::from_str(&format!("\"{}\"", row.get::<_, String>(2)?))
                .unwrap_or(SourceType::Custom("unknown".into())),
            description: row.get(3)?,
            connection_config: json_map(&row.get::<_, String>(4)?),
            created_at: dt(&row.get::<_, String>(5)?),
            updated_at: dt(&row.get::<_, String>(6)?),
        })
    }

    fn row_to_dataset(&self, row: &rusqlite::Row) -> rusqlite::Result<Dataset> {
        let last_crawled: Option<String> = row.get(10)?;
        Ok(Dataset {
            id: uuid(&row.get::<_, String>(0)?),
            data_source_id: uuid(&row.get::<_, String>(1)?),
            name: row.get(2)?,
            physical_name: row.get(3)?,
            dataset_type: serde_json::from_str(&format!("\"{}\"", row.get::<_, String>(4)?))
                .unwrap_or(DatasetType::Table),
            schema: vec![],
            description: row.get(5)?,
            tags: json_vec(&row.get::<_, String>(6)?),
            classification: row.get(7)?,
            location: row.get(8)?,
            row_count: row.get(9)?,
            last_crawled_at: last_crawled.map(|s| dt(&s)),
            version: row.get(13)?,
            metadata: json_map(&row.get::<_, String>(14)?),
            created_at: dt(&row.get::<_, String>(11)?),
            updated_at: dt(&row.get::<_, String>(12)?),
        })
    }

    fn row_to_column(&self, row: &rusqlite::Row) -> rusqlite::Result<Column> {
        Ok(Column {
            id: uuid(&row.get::<_, String>(0)?),
            dataset_id: uuid(&row.get::<_, String>(1)?),
            name: row.get(2)?,
            column_type: row.get(3)?,
            description: row.get(4)?,
            is_nullable: row.get::<_, i32>(5)? != 0,
            is_primary_key: row.get::<_, i32>(6)? != 0,
            is_foreign_key: row.get::<_, i32>(7)? != 0,
            ordinal_position: row.get(8)?,
            classification: row.get(9)?,
            tags: json_vec(&row.get::<_, String>(10)?),
            glossary_term_id: row.get::<_, Option<String>>(11)?.map(|s| uuid(&s)),
            metadata: json_map(&row.get::<_, String>(12)?),
        })
    }

    fn load_columns(&self, conn: &Connection, dataset_id: Uuid) -> CatalogResult<Vec<Column>> {
        let mut stmt = conn
            .prepare("SELECT * FROM columns WHERE dataset_id=?1 ORDER BY ordinal_position")
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![dataset_id.to_string()], |row| {
                self.row_to_column(row)
            })
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| CatalogError::StorageError(e.to_string()))?);
        }
        Ok(result)
    }
}

#[async_trait]
impl CatalogStore for SqliteCatalogStore {
    // ---- DataSources ----
    async fn create_datasource(&self, mut ds: DataSource) -> CatalogResult<DataSource> {
        ds.id = Uuid::now_v7();
        let now = Utc::now();
        ds.created_at = now;
        ds.updated_at = now;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO datasources (id,name,source_type,description,connection_config,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![
                ds.id.to_string(), ds.name, ds.source_type.to_string(),
                ds.description, json_string(&ds.connection_config),
                ds.created_at.to_rfc3339(), ds.updated_at.to_rfc3339()
            ],
        ).map_err(|e| CatalogError::StorageError(format!("Insert datasource: {e}")))?;
        Ok(ds)
    }

    async fn get_datasource(&self, id: Uuid) -> CatalogResult<DataSource> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT * FROM datasources WHERE id=?1",
            rusqlite::params![id.to_string()],
            |row| self.row_to_datasource(row),
        )
        .map_err(|_| CatalogError::NotFound(format!("datasource {id}")))
    }

    async fn list_datasources(&self) -> CatalogResult<PaginatedResponse<DataSource>> {
        let conn = self.conn.lock();
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM datasources", [], |r| r.get(0))
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT * FROM datasources ORDER BY name")
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| self.row_to_datasource(row))
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut data = Vec::new();
        for row in rows {
            data.push(row.map_err(|e| CatalogError::StorageError(e.to_string()))?);
        }
        let total = total as u64;
        Ok(PaginatedResponse {
            data,
            total,
            offset: 0,
            limit: total.max(1),
        })
    }

    async fn update_datasource(&self, mut ds: DataSource) -> CatalogResult<DataSource> {
        ds.updated_at = Utc::now();
        let conn = self.conn.lock();
        let rows = conn
            .execute(
                "UPDATE datasources SET name=?1,source_type=?2,description=?3,connection_config=?4,updated_at=?5 WHERE id=?6",
                rusqlite::params![
                    ds.name, ds.source_type.to_string(), ds.description,
                    json_string(&ds.connection_config), ds.updated_at.to_rfc3339(),
                    ds.id.to_string()
                ],
            )
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        if rows == 0 {
            return Err(CatalogError::NotFound(format!("datasource {}", ds.id)));
        }
        Ok(ds)
    }

    async fn delete_datasource(&self, id: Uuid) -> CatalogResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM datasources WHERE id=?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        Ok(())
    }

    // ---- Datasets ----
    async fn create_dataset(&self, mut ds: Dataset) -> CatalogResult<Dataset> {
        let now = Utc::now();
        ds.id = Uuid::now_v7();
        ds.created_at = now;
        ds.updated_at = now;
        ds.version = 1;
        for col in &mut ds.schema {
            col.id = Uuid::now_v7();
            col.dataset_id = ds.id;
        }
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO datasets (id,data_source_id,name,physical_name,dataset_type,description,tags,classification,location,row_count,last_crawled_at,created_at,updated_at,version,metadata) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            rusqlite::params![
                ds.id.to_string(), ds.data_source_id.to_string(), ds.name, ds.physical_name,
                serde_json::to_string(&ds.dataset_type).unwrap_or_default(),
                ds.description, json_string(&ds.tags), ds.classification,
                ds.location, ds.row_count,
                ds.last_crawled_at.map(|d| d.to_rfc3339()),
                ds.created_at.to_rfc3339(), ds.updated_at.to_rfc3339(),
                ds.version, json_string(&ds.metadata)
            ],
        ).map_err(|e| CatalogError::StorageError(format!("Insert dataset: {e}")))?;

        for col in &ds.schema {
            conn.execute(
                "INSERT INTO columns (id,dataset_id,name,column_type,description,is_nullable,is_primary_key,is_foreign_key,ordinal_position,classification,tags,glossary_term_id,metadata) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
                rusqlite::params![
                    col.id.to_string(), col.dataset_id.to_string(), col.name, col.column_type,
                    col.description, col.is_nullable as i32, col.is_primary_key as i32,
                    col.is_foreign_key as i32, col.ordinal_position, col.classification,
                    json_string(&col.tags), col.glossary_term_id.map(|id| id.to_string()),
                    json_string(&col.metadata)
                ],
            ).map_err(|e| CatalogError::StorageError(format!("Insert column: {e}")))?;
        }

        let schema_version = SchemaVersion {
            dataset_id: ds.id,
            version: ds.version,
            schema: ds.schema.clone(),
            diff_from_previous: None,
            created_at: now,
        };
        conn.execute(
            "INSERT INTO schema_versions (dataset_id, version, schema_json, diff_from_previous, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                schema_version.dataset_id.to_string(),
                schema_version.version,
                json_string(&schema_version.schema),
                schema_version.diff_from_previous,
                schema_version.created_at.to_rfc3339()
            ],
        ).map_err(|e| CatalogError::StorageError(format!("Insert schema version: {e}")))?;

        Ok(ds)
    }

    async fn get_dataset(&self, id: Uuid) -> CatalogResult<Dataset> {
        let conn = self.conn.lock();
        let mut ds = conn
            .query_row(
                "SELECT * FROM datasets WHERE id=?1",
                rusqlite::params![id.to_string()],
                |row| self.row_to_dataset(row),
            )
            .map_err(|_| CatalogError::NotFound(format!("dataset {id}")))?;

        ds.schema = self.load_columns(&conn, id)?;
        Ok(ds)
    }

    async fn get_dataset_by_name(&self, name: &str) -> CatalogResult<Dataset> {
        let uuid_str = {
            let conn = self.conn.lock();
            conn.query_row(
                "SELECT id FROM datasets WHERE name=?1",
                rusqlite::params![name],
                |row| row.get::<_, String>(0),
            )
            .map_err(|_| CatalogError::NotFound(format!("dataset '{name}'")))?
        };
        self.get_dataset(uuid(&uuid_str)).await
    }

    async fn list_datasets(
        &self,
        datasource_id: Option<Uuid>,
        pagination: &PaginationParams,
    ) -> CatalogResult<PaginatedResponse<Dataset>> {
        let conn = self.conn.lock();
        let (total, data) = match datasource_id {
            Some(id) => {
                let total: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM datasets WHERE data_source_id=?1",
                        rusqlite::params![id.to_string()],
                        |r| r.get(0),
                    )
                    .map_err(|e| CatalogError::StorageError(e.to_string()))?;
                let mut stmt = conn
                    .prepare(
                        "SELECT * FROM datasets WHERE data_source_id=?1 ORDER BY name LIMIT ?2 OFFSET ?3",
                    )
                    .map_err(|e| CatalogError::StorageError(e.to_string()))?;
                let rows = stmt
                    .query_map(
                        rusqlite::params![
                            id.to_string(),
                            pagination.limit as i64,
                            pagination.offset as i64
                        ],
                        |row| self.row_to_dataset(row),
                    )
                    .map_err(|e| CatalogError::StorageError(e.to_string()))?;
                let mut result = Vec::new();
                for row in rows {
                    result.push(row.map_err(|e| CatalogError::StorageError(e.to_string()))?);
                }
                (total as u64, result)
            }
            None => {
                let total: i64 = conn
                    .query_row("SELECT COUNT(*) FROM datasets", [], |r| r.get(0))
                    .map_err(|e| CatalogError::StorageError(e.to_string()))?;
                let mut stmt = conn
                    .prepare("SELECT * FROM datasets ORDER BY name LIMIT ?1 OFFSET ?2")
                    .map_err(|e| CatalogError::StorageError(e.to_string()))?;
                let rows = stmt
                    .query_map(
                        rusqlite::params![pagination.limit as i64, pagination.offset as i64],
                        |row| self.row_to_dataset(row),
                    )
                    .map_err(|e| CatalogError::StorageError(e.to_string()))?;
                let mut result = Vec::new();
                for row in rows {
                    result.push(row.map_err(|e| CatalogError::StorageError(e.to_string()))?);
                }
                (total as u64, result)
            }
        };
        Ok(PaginatedResponse {
            data,
            total,
            offset: pagination.offset,
            limit: pagination.limit,
        })
    }

    async fn search_datasets(
        &self,
        query: &str,
        pagination: &PaginationParams,
    ) -> CatalogResult<PaginatedResponse<Dataset>> {
        let q = format!("%{}%", query);
        let conn = self.conn.lock();
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM datasets WHERE name LIKE ?1 OR description LIKE ?1",
                rusqlite::params![q],
                |r| r.get(0),
            )
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT * FROM datasets WHERE name LIKE ?1 OR description LIKE ?1 ORDER BY name LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![q, pagination.limit as i64, pagination.offset as i64],
                |row| self.row_to_dataset(row),
            )
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut data = Vec::new();
        for row in rows {
            data.push(row.map_err(|e| CatalogError::StorageError(e.to_string()))?);
        }
        Ok(PaginatedResponse {
            data,
            total: total as u64,
            offset: pagination.offset,
            limit: pagination.limit,
        })
    }

    async fn update_dataset(&self, mut ds: Dataset) -> CatalogResult<Dataset> {
        ds.updated_at = Utc::now();
        let conn = self.conn.lock();

        let prev_version: i32 = conn
            .query_row(
                "SELECT version FROM datasets WHERE id=?1",
                rusqlite::params![ds.id.to_string()],
                |r| r.get(0),
            )
            .map_err(|_| CatalogError::NotFound(format!("dataset {}", ds.id)))?;

        let prev_schema_json: Option<String> = {
            let mut stmt = conn
                .prepare(
                    "SELECT schema_json FROM schema_versions WHERE dataset_id=?1 AND version=?2",
                )
                .map_err(|e| CatalogError::StorageError(e.to_string()))?;
            stmt.query_row(rusqlite::params![ds.id.to_string(), prev_version], |r| {
                r.get::<_, String>(0)
            })
            .ok()
        };

        ds.version = prev_version + 1;

        conn.execute(
            "UPDATE datasets SET name=?1,physical_name=?2,dataset_type=?3,description=?4,tags=?5,classification=?6,location=?7,row_count=?8,updated_at=?9,version=?10,metadata=?11 WHERE id=?12",
            rusqlite::params![
                ds.name, ds.physical_name,
                serde_json::to_string(&ds.dataset_type).unwrap_or_default(),
                ds.description, json_string(&ds.tags), ds.classification,
                ds.location, ds.row_count, ds.updated_at.to_rfc3339(),
                ds.version, json_string(&ds.metadata), ds.id.to_string()
            ],
        ).map_err(|e| CatalogError::StorageError(e.to_string()))?;

        conn.execute(
            "DELETE FROM columns WHERE dataset_id=?1",
            rusqlite::params![ds.id.to_string()],
        )
        .map_err(|e| CatalogError::StorageError(e.to_string()))?;

        for col in &ds.schema {
            conn.execute(
                "INSERT INTO columns (id,dataset_id,name,column_type,description,is_nullable,is_primary_key,is_foreign_key,ordinal_position,classification,tags,glossary_term_id,metadata) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
                rusqlite::params![
                    col.id.to_string(), col.dataset_id.to_string(), col.name, col.column_type,
                    col.description, col.is_nullable as i32, col.is_primary_key as i32,
                    col.is_foreign_key as i32, col.ordinal_position, col.classification,
                    json_string(&col.tags), col.glossary_term_id.map(|id| id.to_string()),
                    json_string(&col.metadata)
                ],
            ).map_err(|e| CatalogError::StorageError(format!("Insert column: {e}")))?;
        }

        let diff = match prev_schema_json {
            Some(ref json) => {
                let prev_cols: Vec<Column> = serde_json::from_str(json).unwrap_or_default();
                compute_schema_diff(&prev_cols, &ds.schema)
            }
            None => "initial version".to_string(),
        };

        conn.execute(
            "INSERT INTO schema_versions (dataset_id, version, schema_json, diff_from_previous, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                ds.id.to_string(),
                ds.version,
                json_string(&ds.schema),
                diff,
                ds.updated_at.to_rfc3339()
            ],
        ).map_err(|e| CatalogError::StorageError(format!("Insert schema version: {e}")))?;

        Ok(ds)
    }

    async fn delete_dataset(&self, id: Uuid) -> CatalogResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM columns WHERE dataset_id=?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        conn.execute(
            "DELETE FROM datasets WHERE id=?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        conn.execute(
            "DELETE FROM metadata WHERE dataset_id=?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        conn.execute(
            "DELETE FROM schema_versions WHERE dataset_id=?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        Ok(())
    }

    // ---- Columns ----
    async fn get_columns(&self, dataset_id: Uuid) -> CatalogResult<Vec<Column>> {
        let conn = self.conn.lock();
        self.load_columns(&conn, dataset_id)
    }

    async fn update_column(&self, col: Column) -> CatalogResult<Column> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE columns SET name=?1,column_type=?2,description=?3,is_nullable=?4,classification=?5,tags=?6,glossary_term_id=?7,metadata=?8 WHERE id=?9",
            rusqlite::params![
                col.name, col.column_type, col.description,
                col.is_nullable as i32, col.classification,
                json_string(&col.tags),
                col.glossary_term_id.map(|id| id.to_string()),
                json_string(&col.metadata),
                col.id.to_string()
            ],
        ).map_err(|e| CatalogError::StorageError(e.to_string()))?;
        Ok(col)
    }

    // ---- Metadata ----
    async fn set_metadata(&self, entry: MetadataEntry) -> CatalogResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO metadata (dataset_id, column_id, key, value) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                entry.dataset_id.to_string(),
                entry.column_id.map(|id| id.to_string()),
                entry.key,
                entry.value
            ],
        ).map_err(|e| CatalogError::StorageError(format!("Insert metadata: {e}")))?;
        Ok(())
    }

    async fn get_metadata(
        &self,
        dataset_id: Uuid,
        column_id: Option<Uuid>,
    ) -> CatalogResult<HashMap<String, String>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT key, value FROM metadata WHERE dataset_id=?1 AND (column_id=?2 OR (column_id IS NULL AND ?2 IS NULL))",
            )
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![dataset_id.to_string(), column_id.map(|id| id.to_string())],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut map = HashMap::new();
        for row in rows {
            let (k, v) = row.map_err(|e| CatalogError::StorageError(e.to_string()))?;
            map.insert(k, v);
        }
        Ok(map)
    }

    async fn delete_metadata(
        &self,
        dataset_id: Uuid,
        key: &str,
        _column_id: Option<Uuid>,
    ) -> CatalogResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM metadata WHERE dataset_id=?1 AND key=?2",
            rusqlite::params![dataset_id.to_string(), key],
        )
        .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        Ok(())
    }

    // ---- Audit ----
    async fn append_audit_entry(&self, entry: AuditEntry) -> CatalogResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO audit_log (id, timestamp, method, path, status_code, duration_ms, user, api_key) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                entry.id.to_string(),
                entry.timestamp.to_rfc3339(),
                entry.method,
                entry.path,
                entry.status_code as i32,
                entry.duration_ms as i64,
                entry.user,
                entry.api_key
            ],
        ).map_err(|e| CatalogError::StorageError(format!("Insert audit entry: {e}")))?;
        Ok(())
    }

    async fn list_audit_entries(
        &self,
        pagination: &PaginationParams,
    ) -> CatalogResult<PaginatedResponse<AuditEntry>> {
        let conn = self.conn.lock();
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0))
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT * FROM audit_log ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2")
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![pagination.limit as i64, pagination.offset as i64],
                |row| {
                    Ok(AuditEntry {
                        id: uuid(&row.get::<_, String>(0)?),
                        timestamp: dt(&row.get::<_, String>(1)?),
                        method: row.get(2)?,
                        path: row.get(3)?,
                        status_code: row.get::<_, i32>(4)? as u16,
                        duration_ms: row.get::<_, i64>(5)? as u64,
                        user: row.get(6)?,
                        api_key: row.get(7)?,
                    })
                },
            )
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut data = Vec::new();
        for row in rows {
            data.push(row.map_err(|e| CatalogError::StorageError(e.to_string()))?);
        }
        Ok(PaginatedResponse {
            data,
            total: total as u64,
            offset: pagination.offset,
            limit: pagination.limit,
        })
    }

    // ---- Schema Versions ----
    async fn get_schema_versions(&self, dataset_id: Uuid) -> CatalogResult<Vec<SchemaVersion>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT dataset_id, version, schema_json, diff_from_previous, created_at FROM schema_versions WHERE dataset_id=?1 ORDER BY version",
            )
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![dataset_id.to_string()], |row| {
                Ok(SchemaVersion {
                    dataset_id: uuid(&row.get::<_, String>(0)?),
                    version: row.get(1)?,
                    schema: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                    diff_from_previous: row.get(3)?,
                    created_at: dt(&row.get::<_, String>(4)?),
                })
            })
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| CatalogError::StorageError(e.to_string()))?);
        }
        Ok(result)
    }

    async fn diff_schema(
        &self,
        dataset_id: Uuid,
        from_version: i32,
        to_version: i32,
    ) -> CatalogResult<String> {
        let versions = self.get_schema_versions(dataset_id).await?;
        let from = versions
            .iter()
            .find(|v| v.version == from_version)
            .ok_or_else(|| CatalogError::NotFound(format!("schema version {from_version}")))?
            .schema
            .clone();
        let to = versions
            .iter()
            .find(|v| v.version == to_version)
            .ok_or_else(|| CatalogError::NotFound(format!("schema version {to_version}")))?
            .schema
            .clone();
        Ok(compute_schema_diff(&from, &to))
    }

    // ---- Lineage (stub - full graph in SQLite requires dedicated edge table) ----
    async fn add_lineage_node(&self, _node: LineageNode) -> CatalogResult<LineageNode> {
        Err(CatalogError::StorageError(
            "Lineage graph not yet implemented in SQLite store".into(),
        ))
    }
    async fn add_lineage_edge(&self, _edge: LineageEdge) -> CatalogResult<LineageEdge> {
        Err(CatalogError::StorageError(
            "Lineage graph not yet implemented in SQLite store".into(),
        ))
    }
    async fn get_lineage(
        &self,
        _dataset_id: Uuid,
        _direction: LineageDirection,
    ) -> CatalogResult<LineageGraph> {
        Ok(LineageGraph {
            nodes: vec![],
            edges: vec![],
        })
    }
    async fn get_column_lineage(
        &self,
        _dataset_id: Uuid,
        _column_name: &str,
    ) -> CatalogResult<Vec<ColumnLineageInfo>> {
        Ok(vec![])
    }

    async fn ingest_openlineage_event(&self, event: OpenLineageEvent) -> CatalogResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO openlineage_events (event_json, ingested_at) VALUES (?1, ?2)",
            rusqlite::params![
                serde_json::to_string(&event).unwrap_or_default(),
                Utc::now().to_rfc3339()
            ],
        )
        .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        Ok(())
    }

    // ---- Glossary ----
    async fn create_glossary_term(&self, mut term: GlossaryTerm) -> CatalogResult<GlossaryTerm> {
        term.id = Uuid::now_v7();
        let now = Utc::now();
        term.created_at = now;
        term.updated_at = now;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO glossary_terms (id,name,description,short_description,domain,synonyms,related_term_ids,custom_properties,status,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            rusqlite::params![
                term.id.to_string(), term.name, term.description, term.short_description,
                term.domain, json_string(&term.synonyms), json_string(&term.related_term_ids),
                json_string(&term.custom_properties),
                serde_json::to_string(&term.status).unwrap_or_default(),
                term.created_at.to_rfc3339(), term.updated_at.to_rfc3339()
            ],
        ).map_err(|e| CatalogError::StorageError(format!("Insert term: {e}")))?;
        Ok(term)
    }

    async fn get_glossary_term(&self, id: Uuid) -> CatalogResult<GlossaryTerm> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT * FROM glossary_terms WHERE id=?1",
            rusqlite::params![id.to_string()],
            |row| {
                Ok(GlossaryTerm {
                    id: uuid(row.get::<_, String>(0)?.as_str()),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    short_description: row.get(3)?,
                    domain: row.get(4)?,
                    synonyms: json_vec(&row.get::<_, String>(5)?),
                    related_term_ids: json_vec(&row.get::<_, String>(6)?),
                    custom_properties: json_map(&row.get::<_, String>(7)?),
                    status: serde_json::from_str(&row.get::<_, String>(8)?)
                        .unwrap_or(TermStatus::Draft),
                    created_at: dt(&row.get::<_, String>(9)?),
                    updated_at: dt(&row.get::<_, String>(10)?),
                })
            },
        )
        .map_err(|_| CatalogError::NotFound(format!("glossary term {id}")))
    }

    async fn list_glossary_terms(
        &self,
        pagination: &PaginationParams,
    ) -> CatalogResult<PaginatedResponse<GlossaryTerm>> {
        let conn = self.conn.lock();
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM glossary_terms", [], |r| r.get(0))
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT * FROM glossary_terms ORDER BY name LIMIT ?1 OFFSET ?2")
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![pagination.limit as i64, pagination.offset as i64],
                |row| {
                    Ok(GlossaryTerm {
                        id: uuid(row.get::<_, String>(0)?.as_str()),
                        name: row.get(1)?,
                        description: row.get(2)?,
                        short_description: row.get(3)?,
                        domain: row.get(4)?,
                        synonyms: json_vec(&row.get::<_, String>(5)?),
                        related_term_ids: json_vec(&row.get::<_, String>(6)?),
                        custom_properties: json_map(&row.get::<_, String>(7)?),
                        status: serde_json::from_str(&row.get::<_, String>(8)?)
                            .unwrap_or(TermStatus::Draft),
                        created_at: dt(&row.get::<_, String>(9)?),
                        updated_at: dt(&row.get::<_, String>(10)?),
                    })
                },
            )
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut data = Vec::new();
        for row in rows {
            data.push(row.map_err(|e| CatalogError::StorageError(e.to_string()))?);
        }
        Ok(PaginatedResponse {
            data,
            total: total as u64,
            offset: pagination.offset,
            limit: pagination.limit,
        })
    }

    async fn update_glossary_term(&self, mut term: GlossaryTerm) -> CatalogResult<GlossaryTerm> {
        term.updated_at = Utc::now();
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE glossary_terms SET name=?1,description=?2,short_description=?3,domain=?4,synonyms=?5,related_term_ids=?6,custom_properties=?7,status=?8,updated_at=?9 WHERE id=?10",
            rusqlite::params![
                term.name, term.description, term.short_description, term.domain,
                json_string(&term.synonyms), json_string(&term.related_term_ids),
                json_string(&term.custom_properties),
                serde_json::to_string(&term.status).unwrap_or_default(),
                term.updated_at.to_rfc3339(), term.id.to_string()
            ],
        ).map_err(|e| CatalogError::StorageError(e.to_string()))?;
        Ok(term)
    }

    async fn delete_glossary_term(&self, id: Uuid) -> CatalogResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM term_mappings WHERE term_id=?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        conn.execute(
            "DELETE FROM glossary_terms WHERE id=?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn create_term_mapping(&self, mut mapping: TermMapping) -> CatalogResult<TermMapping> {
        mapping.id = Uuid::now_v7();
        mapping.created_at = Utc::now();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO term_mappings (id,term_id,dataset_id,column_id,created_at) VALUES (?1,?2,?3,?4,?5)",
            rusqlite::params![
                mapping.id.to_string(), mapping.term_id.to_string(),
                mapping.dataset_id.to_string(),
                mapping.column_id.map(|id| id.to_string()),
                mapping.created_at.to_rfc3339()
            ],
        ).map_err(|e| CatalogError::StorageError(e.to_string()))?;
        Ok(mapping)
    }

    async fn get_term_mappings(&self, term_id: Uuid) -> CatalogResult<Vec<TermMapping>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT * FROM term_mappings WHERE term_id=?1")
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![term_id.to_string()], |row| {
                Ok(TermMapping {
                    id: uuid(row.get::<_, String>(0)?.as_str()),
                    term_id: uuid(row.get::<_, String>(1)?.as_str()),
                    dataset_id: uuid(row.get::<_, String>(2)?.as_str()),
                    column_id: row.get::<_, Option<String>>(3)?.map(|s| uuid(&s)),
                    created_at: dt(&row.get::<_, String>(4)?),
                })
            })
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| CatalogError::StorageError(e.to_string()))?);
        }
        Ok(result)
    }

    // ---- Policies ----
    async fn create_policy(&self, mut policy: Policy) -> CatalogResult<Policy> {
        policy.id = Uuid::now_v7();
        let now = Utc::now();
        policy.created_at = now;
        policy.updated_at = now;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO policies (id,name,description,policy_type,rules,enabled,priority,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            rusqlite::params![
                policy.id.to_string(), policy.name, policy.description,
                serde_json::to_string(&policy.policy_type).unwrap_or_default(),
                json_string(&policy.rules), policy.enabled as i32, policy.priority,
                policy.created_at.to_rfc3339(), policy.updated_at.to_rfc3339()
            ],
        ).map_err(|e| CatalogError::StorageError(format!("Insert policy: {e}")))?;
        Ok(policy)
    }

    async fn get_policy(&self, id: Uuid) -> CatalogResult<Policy> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT * FROM policies WHERE id=?1",
            rusqlite::params![id.to_string()],
            |row| {
                Ok(Policy {
                    id: uuid(row.get::<_, String>(0)?.as_str()),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    policy_type: serde_json::from_str(&row.get::<_, String>(3)?)
                        .unwrap_or(PolicyType::Access),
                    rules: json_vec(&row.get::<_, String>(4)?),
                    enabled: row.get::<_, i32>(5)? != 0,
                    priority: row.get(6)?,
                    created_at: dt(&row.get::<_, String>(7)?),
                    updated_at: dt(&row.get::<_, String>(8)?),
                })
            },
        )
        .map_err(|_| CatalogError::NotFound(format!("policy {id}")))
    }

    async fn list_policies(
        &self,
        pagination: &PaginationParams,
    ) -> CatalogResult<PaginatedResponse<Policy>> {
        let conn = self.conn.lock();
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM policies", [], |r| r.get(0))
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT * FROM policies ORDER BY name LIMIT ?1 OFFSET ?2")
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![pagination.limit as i64, pagination.offset as i64],
                |row| {
                    Ok(Policy {
                        id: uuid(row.get::<_, String>(0)?.as_str()),
                        name: row.get(1)?,
                        description: row.get(2)?,
                        policy_type: serde_json::from_str(&row.get::<_, String>(3)?)
                            .unwrap_or(PolicyType::Access),
                        rules: json_vec(&row.get::<_, String>(4)?),
                        enabled: row.get::<_, i32>(5)? != 0,
                        priority: row.get(6)?,
                        created_at: dt(&row.get::<_, String>(7)?),
                        updated_at: dt(&row.get::<_, String>(8)?),
                    })
                },
            )
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut data = Vec::new();
        for row in rows {
            data.push(row.map_err(|e| CatalogError::StorageError(e.to_string()))?);
        }
        Ok(PaginatedResponse {
            data,
            total: total as u64,
            offset: pagination.offset,
            limit: pagination.limit,
        })
    }

    async fn update_policy(&self, mut policy: Policy) -> CatalogResult<Policy> {
        policy.updated_at = Utc::now();
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE policies SET name=?1,description=?2,policy_type=?3,rules=?4,enabled=?5,priority=?6,updated_at=?7 WHERE id=?8",
            rusqlite::params![
                policy.name, policy.description,
                serde_json::to_string(&policy.policy_type).unwrap_or_default(),
                json_string(&policy.rules), policy.enabled as i32, policy.priority,
                policy.updated_at.to_rfc3339(), policy.id.to_string()
            ],
        ).map_err(|e| CatalogError::StorageError(e.to_string()))?;
        Ok(policy)
    }

    async fn delete_policy(&self, id: Uuid) -> CatalogResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM policies WHERE id=?1",
            rusqlite::params![id.to_string()],
        )
        .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn get_active_policies(&self) -> CatalogResult<Vec<Policy>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT * FROM policies WHERE enabled=1 ORDER BY priority")
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Policy {
                    id: uuid(row.get::<_, String>(0)?.as_str()),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    policy_type: serde_json::from_str(&row.get::<_, String>(3)?)
                        .unwrap_or(PolicyType::Access),
                    rules: json_vec(&row.get::<_, String>(4)?),
                    enabled: row.get::<_, i32>(5)? != 0,
                    priority: row.get(6)?,
                    created_at: dt(&row.get::<_, String>(7)?),
                    updated_at: dt(&row.get::<_, String>(8)?),
                })
            })
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| CatalogError::StorageError(e.to_string()))?);
        }
        Ok(result)
    }

    // ---- Crawls ----
    async fn create_crawl_run(&self, mut run: CrawlRun) -> CatalogResult<CrawlRun> {
        run.id = Uuid::now_v7();
        run.started_at = Utc::now();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO crawl_runs (id,data_source_id,status,started_at,completed_at,datasets_found,events_processed,error) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                run.id.to_string(), run.data_source_id.to_string(),
                serde_json::to_string(&run.status).unwrap_or_default(),
                run.started_at.to_rfc3339(),
                run.completed_at.map(|d| d.to_rfc3339()),
                run.datasets_found, run.events_processed, run.error
            ],
        ).map_err(|e| CatalogError::StorageError(format!("Insert crawl: {e}")))?;
        Ok(run)
    }

    async fn update_crawl_run(&self, run: CrawlRun) -> CatalogResult<CrawlRun> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE crawl_runs SET status=?1,completed_at=?2,datasets_found=?3,events_processed=?4,error=?5 WHERE id=?6",
            rusqlite::params![
                serde_json::to_string(&run.status).unwrap_or_default(),
                run.completed_at.map(|d| d.to_rfc3339()),
                run.datasets_found, run.events_processed, run.error,
                run.id.to_string()
            ],
        ).map_err(|e| CatalogError::StorageError(e.to_string()))?;
        Ok(run)
    }

    async fn list_crawl_runs(
        &self,
        datasource_id: Uuid,
        pagination: &PaginationParams,
    ) -> CatalogResult<PaginatedResponse<CrawlRun>> {
        let conn = self.conn.lock();
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM crawl_runs WHERE data_source_id=?1",
                rusqlite::params![datasource_id.to_string()],
                |r| r.get(0),
            )
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT * FROM crawl_runs WHERE data_source_id=?1 ORDER BY started_at DESC LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map(
                rusqlite::params![
                    datasource_id.to_string(),
                    pagination.limit as i64,
                    pagination.offset as i64
                ],
                |row| {
                    Ok(CrawlRun {
                        id: uuid(row.get::<_, String>(0)?.as_str()),
                        data_source_id: uuid(row.get::<_, String>(1)?.as_str()),
                        status: serde_json::from_str(&row.get::<_, String>(2)?)
                            .unwrap_or(CrawlStatus::Failed),
                        started_at: dt(&row.get::<_, String>(3)?),
                        completed_at: row.get::<_, Option<String>>(4)?.map(|s| dt(&s)),
                        datasets_found: row.get(5)?,
                        events_processed: row.get(6)?,
                        error: row.get(7)?,
                    })
                },
            )
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let mut data = Vec::new();
        for row in rows {
            data.push(row.map_err(|e| CatalogError::StorageError(e.to_string()))?);
        }
        Ok(PaginatedResponse {
            data,
            total: total as u64,
            offset: pagination.offset,
            limit: pagination.limit,
        })
    }

    // ---- Search ----
    async fn search(&self, query: &str, limit: usize) -> CatalogResult<SearchResults> {
        let q = format!("%{}%", query.to_lowercase());
        let conn = self.conn.lock();
        let mut results = Vec::new();

        if let Ok(mut stmt) = conn.prepare(
            "SELECT id, name, description FROM datasets WHERE LOWER(name) LIKE ?1 OR LOWER(description) LIKE ?1 LIMIT ?2",
        )
            && let Ok(rows) = stmt.query_map(rusqlite::params![q, limit as i64], |row| {
                Ok(SearchResult {
                    dataset_id: Some(uuid(row.get::<_, String>(0)?.as_str())),
                    column_id: None,
                    glossary_term_id: None,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    score: 0.8,
                    kind: "dataset".into(),
                })
            })
        {
            for row in rows.flatten() {
                results.push(row);
            }
        }

        if let Ok(mut stmt) = conn.prepare(
            "SELECT id, name, description FROM glossary_terms WHERE LOWER(name) LIKE ?1 OR LOWER(description) LIKE ?1 LIMIT ?2",
        )
            && let Ok(rows) = stmt.query_map(rusqlite::params![q, limit as i64], |row| {
                Ok(SearchResult {
                    dataset_id: None,
                    column_id: None,
                    glossary_term_id: Some(uuid(row.get::<_, String>(0)?.as_str())),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    score: 0.9,
                    kind: "glossary_term".into(),
                })
            })
        {
            for row in rows.flatten() {
                results.push(row);
            }
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        let total = results.len();
        Ok(SearchResults { results, total })
    }

    // ---- Metrics ----
    async fn get_metrics(&self) -> CatalogResult<MetricsSnapshot> {
        let conn = self.conn.lock();

        let datasource_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM datasources", [], |r| r.get(0))
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let dataset_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM datasets", [], |r| r.get(0))
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let column_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM columns", [], |r| r.get(0))
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let glossary_term_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM glossary_terms", [], |r| r.get(0))
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let policy_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM policies", [], |r| r.get(0))
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let crawl_run_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM crawl_runs", [], |r| r.get(0))
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;
        let total_api_calls: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0))
            .map_err(|e| CatalogError::StorageError(e.to_string()))?;

        Ok(MetricsSnapshot {
            datasource_count: datasource_count as usize,
            dataset_count: dataset_count as usize,
            column_count: column_count as usize,
            glossary_term_count: glossary_term_count as usize,
            policy_count: policy_count as usize,
            lineage_edge_count: 0,
            crawl_run_count: crawl_run_count as usize,
            total_api_calls: total_api_calls as u64,
            uptime_seconds: 0,
        })
    }
}
