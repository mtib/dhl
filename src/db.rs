use anyhow::Result;
use rocksdb::{DB, Options};
use std::path::PathBuf;

pub struct Store {
    db: DB,
}

const ROOTS_CF: &str = "roots";
const WORKSPACES_CF: &str = "workspaces";

impl Store {
    pub fn open(path: PathBuf) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db = DB::open_cf(&opts, path, [ROOTS_CF, WORKSPACES_CF])?;
        Ok(Self { db })
    }

    fn roots_cf(&self) -> &rocksdb::ColumnFamily {
        self.db.cf_handle(ROOTS_CF).expect("roots CF missing")
    }

    fn workspaces_cf(&self) -> &rocksdb::ColumnFamily {
        self.db
            .cf_handle(WORKSPACES_CF)
            .expect("workspaces CF missing")
    }

    pub fn add_root(&self, path: &str) -> Result<()> {
        self.db.put_cf(self.roots_cf(), path.as_bytes(), b"1")?;
        Ok(())
    }

    pub fn list_roots(&self) -> Result<Vec<String>> {
        let iter = self
            .db
            .iterator_cf(self.roots_cf(), rocksdb::IteratorMode::Start);
        let mut roots = Vec::new();
        for item in iter {
            let (key, _) = item?;
            roots.push(String::from_utf8(key.to_vec())?);
        }
        Ok(roots)
    }

    pub fn remove_root(&self, path: &str) -> Result<()> {
        self.db.delete_cf(self.roots_cf(), path.as_bytes())?;
        Ok(())
    }

    pub fn put_workspace(&self, name: &str, value: &str) -> Result<()> {
        self.db
            .put_cf(self.workspaces_cf(), name.as_bytes(), value.as_bytes())?;
        Ok(())
    }

    pub fn get_workspace(&self, name: &str) -> Result<Option<String>> {
        match self.db.get_cf(self.workspaces_cf(), name.as_bytes())? {
            Some(v) => Ok(Some(String::from_utf8(v.to_vec())?)),
            None => Ok(None),
        }
    }

    pub fn delete_workspace(&self, name: &str) -> Result<()> {
        self.db.delete_cf(self.workspaces_cf(), name.as_bytes())?;
        Ok(())
    }

    pub fn list_workspaces(&self) -> Result<Vec<(String, String)>> {
        let iter = self
            .db
            .iterator_cf(self.workspaces_cf(), rocksdb::IteratorMode::Start);
        let mut workspaces = Vec::new();
        for item in iter {
            let (key, val) = item?;
            let name = String::from_utf8(key.to_vec())?;
            let value = String::from_utf8(val.to_vec())?;
            workspaces.push((name, value));
        }
        Ok(workspaces)
    }
}
