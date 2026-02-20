use ark_bls12_381::Fr;
use ark_ff::{BigInteger, PrimeField};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS leaves (
                idx INTEGER PRIMARY KEY,
                commitment BLOB NOT NULL,
                block_height INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sync_cursor (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                last_ledger INTEGER NOT NULL,
                last_cursor TEXT
            );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_leaf(&self, idx: usize, commitment: Fr, block_height: u64) -> rusqlite::Result<()> {
        let bytes = fr_to_bytes(&commitment);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO leaves (idx, commitment, block_height) VALUES (?1, ?2, ?3)",
            params![idx as i64, bytes, block_height as i64],
        )?;
        Ok(())
    }

    pub fn load_leaves(&self) -> rusqlite::Result<Vec<Fr>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT commitment FROM leaves ORDER BY idx")?;
        let leaves = stmt
            .query_map([], |row| {
                let bytes: Vec<u8> = row.get(0)?;
                Ok(fr_from_bytes(&bytes))
            })?
            .collect::<rusqlite::Result<Vec<Fr>>>()?;
        Ok(leaves)
    }

    pub fn get_leaf_by_commitment(&self, commitment: Fr) -> rusqlite::Result<Option<(usize, u64)>> {
        let bytes = fr_to_bytes(&commitment);
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT idx, block_height FROM leaves WHERE commitment = ?1",
        )?;
        let mut rows = stmt.query_map(params![bytes], |row| {
            let idx: i64 = row.get(0)?;
            let height: i64 = row.get(1)?;
            Ok((idx as usize, height as u64))
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn save_cursor(&self, last_ledger: u64, cursor: Option<&str>) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sync_cursor (id, last_ledger, last_cursor)
             VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET last_ledger = ?1, last_cursor = ?2",
            params![last_ledger as i64, cursor],
        )?;
        Ok(())
    }

    pub fn load_cursor(&self) -> rusqlite::Result<Option<(u64, Option<String>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT last_ledger, last_cursor FROM sync_cursor WHERE id = 1",
        )?;
        let mut rows = stmt.query_map([], |row| {
            let ledger: i64 = row.get(0)?;
            let cursor: Option<String> = row.get(1)?;
            Ok((ledger as u64, cursor))
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }
}

fn fr_to_bytes(fr: &Fr) -> Vec<u8> {
    fr.into_bigint().to_bytes_be()
}

fn fr_from_bytes(bytes: &[u8]) -> Fr {
    Fr::from_be_bytes_mod_order(bytes)
}
