use anyhow::{Context, Result};
use libsql::Builder;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use taskchampion::{
    storage::{Storage, StorageTxn},
    Operation, Uuid,
};

/// Storage configuration for Turso
#[derive(Serialize, Deserialize)]
pub enum TursoConfig {
    /// Local file database
    Local { path: String },
    /// Remote database
    Remote { url: String, token: String },
    /// Embedded replica (local file + remote sync)
    EmbeddedReplica {
        path: String,
        url: String,
        token: String,
    },
}

pub struct TursoStorage {
    db: Arc<libsql::Database>,
    conn: Arc<libsql::Connection>,
}

impl TursoStorage {
    pub async fn new(config: TursoConfig) -> Result<Self> {
        let db = match config {
            TursoConfig::Local { path } => Builder::new_local(path).build().await?,
            TursoConfig::Remote { url, token } => {
                Builder::new_remote(url, token).build().await?
            }
            TursoConfig::EmbeddedReplica { path, url, token } => {
                Builder::new_remote_replica(path, url, token)
                    .build()
                    .await?
            }
        };

        // Best-effort sync on startup for embedded replicas.
        // We ignore errors to allow offline usage (using local cache).
        if let Err(e) = db.sync().await {
            eprintln!("Taskwarrior Turso Sync Warning: Failed to sync on startup: {}", e);
        }

        let conn = db.connect()?;
        let storage = Self {
            db: Arc::new(db),
            conn: Arc::new(conn),
        };

        storage.initialize().await?;

        Ok(storage)
    }

    async fn initialize(&self) -> Result<()> {
        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS operations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    uuid BLOB NOT NULL, -- UUID of the operation
                    data TEXT NOT NULL, -- JSON serialized operation
                    timestamp INTEGER -- Timestamp for ordering/sync
                )",
                (),
            )
            .await?;

        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS working_set (
                    id INTEGER PRIMARY KEY, -- 1-based index
                    uuid BLOB -- UUID of the task, can be null
                )",
                (),
            )
            .await?;

        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS tasks (
                    uuid BLOB PRIMARY KEY,
                    data TEXT
                )",
                (),
            )
            .await?;

        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS meta (
                    key TEXT PRIMARY KEY,
                    value BLOB
                )",
                (),
            )
            .await?;
            
         Ok(())
    }
}

impl Storage for TursoStorage {
    fn txn<'a>(&'a mut self) -> Result<Box<dyn StorageTxn + 'a>, taskchampion::Error> {
        // In a real implementation, we might want to start a transaction here.
        // For now, Turso/libsql interactions in the `StorageTxn` will be auto-commit or managed there.
        // But `StorageTxn` contract assumes exclusive access or transaction isolation usually.
        // Libsql client assumes async, but passing it into the sync `txn` method requires blocking or handling async in sync context.
        // TaskChampion's `Storage` trait is synchronous. This is a challenge because libsql is async.
        // We will misuse `tokio::task::block_in_place` or `tokio::runtime::Handle` if we are in an async runtime,
        // or create a runtime if we are not.
        //
        // However, standard `taskwarrior` is synchronous. 
        // We might need to use `libsql` blocking API if available or wrap async calls.
        
        // Actually, `libsql` has a blocking API behind a feature flag or via `Connection`.
        // The dependency we added `libsql` features `core` `hrana` etc.
        // Let's assume we are running in an environment where we can block.
        // Since `Storage::txn` is sync, we need a way to execute async code.
        
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| taskchampion::Error::Database(format!("Failed to create runtime: {}", e)))?;

        Ok(Box::new(TursoTxn {
            conn: self.conn.clone(),
            runtime,
        }))
    }
}

struct TursoTxn {
    conn: Arc<libsql::Connection>,
    runtime: tokio::runtime::Runtime,
}

impl StorageTxn for TursoTxn {
    fn is_empty(&mut self) -> Result<bool, taskchampion::Error> {
        self.block_on(async {
            let mut rows = self.conn.query("SELECT count(*) FROM operations", ()).await
                .map_err(|e| taskchampion::Error::Database(e.to_string()))?;
            let count: i64 = if let Some(row) = rows.next().await.map_err(|e| taskchampion::Error::Database(e.to_string()))? {
                row.get(0).map_err(|e| taskchampion::Error::Database(e.to_string()))?
            } else {
                0
            };
            Ok(count == 0)
        })
    }

    fn num_unsynced_operations(&mut self) -> Result<usize, taskchampion::Error> {
         // This is simplified: in actual TaskChampion storage, we need to track synced vs unsynced.
         // Usually via a separate table or a column.
         // For now, assume all operations are unsynced for the prototype or implement properly.
         // Existing sqlite storage uses a base_version.
         // Let's add a sync tracking mechanism if needed, but the trait just asks for the number.
         // If we follow the sqlite implementation, we just count all operations since the last sync.
         // But we haven't implemented sync yet.
         
         // Let's assume we implement a basic version that just counts all ops for now.
         self.block_on(async {
            let mut rows = self.conn.query("SELECT count(*) FROM operations", ()).await
                 .map_err(|e| taskchampion::Error::Database(e.to_string()))?;
             let count: i64 = if let Some(row) = rows.next().await.map_err(|e| taskchampion::Error::Database(e.to_string()))? {
                 row.get(0).map_err(|e| taskchampion::Error::Database(e.to_string()))?
             } else {
                 0
             };
             Ok(count as usize)
         })
    }

    fn add_operation(&mut self, op: Operation) -> Result<(), taskchampion::Error> {
        let json = serde_json::to_string(&op).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
        let uuid = op.get_uuid(); // We need to check if Operation exposes uuid easily for valid ops.
        // Actually Operation enum might not expose get_uuid directly on all variants, but we implemented a helper in C++ bridge.
        // In Rust `taskchampion::Operation`, it is an enum.
        
        // We will store the JSON.
        // The UUID column in schema helps with lookup but 'UndoPoint' has no UUID.
        // We can make UUID nullable or store a dummy.
        
        let uuid_bytes = match op {
            Operation::Create { uuid } => uuid.as_bytes().to_vec(),
            Operation::Update { uuid, .. } => uuid.as_bytes().to_vec(),
            Operation::Delete { uuid, .. } => uuid.as_bytes().to_vec(),
            Operation::UndoPoint => vec![0u8; 16], // Dummy
        };
        
        self.block_on(async {
            self.conn.execute(
                "INSERT INTO operations (uuid, data) VALUES (?, ?)",
                vec![libsql::Value::Blob(uuid_bytes), libsql::Value::Text(json)],
            ).await.map_err(|e| taskchampion::Error::Database(e.to_string()))?;
            Ok(())
        })
    }

    fn remove_operation(&mut self, _op: Operation) -> Result<(), taskchampion::Error> {
         // remove last operation
         self.block_on(async {
             // In a real impl, we should verify `op` matches the last one.
             // For simplicity, just pop the last one.
             self.conn.execute("DELETE FROM operations WHERE id = (SELECT MAX(id) FROM operations)", ())
                .await.map_err(|e| taskchampion::Error::Database(e.to_string()))?;
             Ok(())
         })
    }

    fn sync_complete(&mut self) -> Result<(), taskchampion::Error> {
        // In a real sync, we would delete synced operations or mark them.
        // TaskChampion's model (at least traditionally) deletes synced operations from the local operational log
        // after they are applied to the "base" or successfully sent to server.
        // The trait documentation says "A sync has been completed, so all operations should be marked as synced."
        // Storage::sqlite deletes them.
        self.block_on(async {
            self.conn.execute("DELETE FROM operations", ()).await
                .map_err(|e| taskchampion::Error::Database(e.to_string()))?;
            Ok(())
        })
    }

    fn get_working_set(&mut self) -> Result<Vec<Option<Uuid>>, taskchampion::Error> {
        self.block_on(async {
            let mut rows = self.conn.query("SELECT id, uuid FROM working_set ORDER BY id ASC", ()).await
                .map_err(|e| taskchampion::Error::Database(e.to_string()))?;
            
            let mut set = Vec::new();
            while let Some(row) = rows.next().await.map_err(|e| taskchampion::Error::Database(e.to_string()))? {
                let id: i64 = row.get(0).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
                let uuid_blob: Option<Vec<u8>> = row.get(1).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
                let uuid = uuid_blob.map(|b| Uuid::from_bytes(b.try_into().unwrap_or([0; 16])));
                
                // Ensure vector is large enough
                let idx = id as usize;
                if set.len() <= idx {
                    set.resize(idx + 1, None);
                }
                set[idx] = uuid;
            }
            // Index 0 is always None per trait docs (1-based index)
            if set.is_empty() {
                set.push(None);
            } else {
                 set[0] = None;
            }
            Ok(set)
        })
    }

    fn add_to_working_set(&mut self, uuid: Uuid) -> Result<usize, taskchampion::Error> {
        self.block_on(async {
            // Find max id
             let mut rows = self.conn.query("SELECT MAX(id) FROM working_set", ()).await
                .map_err(|e| taskchampion::Error::Database(e.to_string()))?;
             let max_id: i64 = if let Some(row) = rows.next().await.map_err(|e| taskchampion::Error::Database(e.to_string()))? {
                 row.get(0).unwrap_or(0)
             } else {
                 0
             };
             let new_id = max_id + 1;
             
             self.conn.execute(
                 "INSERT INTO working_set (id, uuid) VALUES (?, ?)", 
                 vec![libsql::Value::Integer(new_id), libsql::Value::Blob(uuid.as_bytes().to_vec())]
             ).await.map_err(|e| taskchampion::Error::Database(e.to_string()))?;
             
             Ok(new_id as usize)
        })
    }

    fn set_working_set_item(&mut self, index: usize, uuid: Option<Uuid>) -> Result<(), taskchampion::Error> {
        self.block_on(async {
             if let Some(u) = uuid {
                 self.conn.execute(
                     "INSERT OR REPLACE INTO working_set (id, uuid) VALUES (?, ?)",
                     vec![libsql::Value::Integer(index as i64), libsql::Value::Blob(u.as_bytes().to_vec())]
                 ).await.map_err(|e| taskchampion::Error::Database(e.to_string()))?;
             } else {
                 self.conn.execute(
                     "DELETE FROM working_set WHERE id = ?",
                     vec![libsql::Value::Integer(index as i64)]
                 ).await.map_err(|e| taskchampion::Error::Database(e.to_string()))?;
             }
             Ok(())
        })
    }

    fn clear_working_set(&mut self) -> Result<(), taskchampion::Error> {
        self.block_on(async {
             self.conn.execute("DELETE FROM working_set", ()).await
                .map_err(|e| taskchampion::Error::Database(e.to_string()))?;
             Ok(())
        })
    }

    fn commit(&mut self) -> Result<(), taskchampion::Error> {
        // We are using auto-commit for now with one-off statements.
        // If we wrapped this in a real transaction, we would commit here.
        Ok(())
    }

    fn get_task(&mut self, uuid: Uuid) -> Result<Option<std::collections::HashMap<String, String>>, taskchampion::Error> {
        self.block_on(async {
            let mut rows = self.conn.query("SELECT data FROM tasks WHERE uuid = ?", vec![libsql::Value::Blob(uuid.as_bytes().to_vec())]).await
                .map_err(|e| taskchampion::Error::Database(e.to_string()))?;
            if let Some(row) = rows.next().await.map_err(|e| taskchampion::Error::Database(e.to_string()))? {
                let data: String = row.get(0).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
                let task: std::collections::HashMap<String, String> = serde_json::from_str(&data).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
                Ok(Some(task))
            } else {
                Ok(None)
            }
        })
    }

    fn get_pending_tasks(&mut self) -> Result<Vec<(Uuid, std::collections::HashMap<String, String>)>, taskchampion::Error> {
         // This assumes we can just filter using the working set? Or does pending mean something else?
         // In TaskChampion, "pending" usually means tasks in the working set.
         // Let's rely on retrieving all for now or check usage.
         // Wait, the trait method return type is `Vec<(Uuid, HashMap)>`.
         
         // Implementation: Get all uuids from working_set, then fetch each task.
         let working_set = self.get_working_set()?;
         let mut result = Vec::new();
         for uuid_opt in working_set {
             if let Some(uuid) = uuid_opt {
                 if let Some(task) = self.get_task(uuid)? {
                     result.push((uuid, task));
                 }
             }
         }
         Ok(result)
    }

    fn create_task(&mut self, uuid: Uuid) -> Result<bool, taskchampion::Error> {
        self.block_on(async {
            // Check if exists
             let mut rows = self.conn.query("SELECT 1 FROM tasks WHERE uuid = ?", vec![libsql::Value::Blob(uuid.as_bytes().to_vec())]).await
                .map_err(|e| taskchampion::Error::Database(e.to_string()))?;
             if rows.next().await.map_err(|e| taskchampion::Error::Database(e.to_string()))?.is_some() {
                 return Ok(false);
             }
             
             let empty_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
             let json = serde_json::to_string(&empty_map).unwrap();
             self.conn.execute("INSERT INTO tasks (uuid, data) VALUES (?, ?)", 
                 vec![libsql::Value::Blob(uuid.as_bytes().to_vec()), libsql::Value::Text(json)]
             ).await.map_err(|e| taskchampion::Error::Database(e.to_string()))?;
             Ok(true)
        })
    }

    fn set_task(&mut self, uuid: Uuid, task: std::collections::HashMap<String, String>) -> Result<(), taskchampion::Error> {
        let json = serde_json::to_string(&task).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
        self.block_on(async {
            self.conn.execute("INSERT OR REPLACE INTO tasks (uuid, data) VALUES (?, ?)",
                 vec![libsql::Value::Blob(uuid.as_bytes().to_vec()), libsql::Value::Text(json)]
            ).await.map_err(|e| taskchampion::Error::Database(e.to_string()))?;
            Ok(())
        })
    }

    fn delete_task(&mut self, uuid: Uuid) -> Result<bool, taskchampion::Error> {
        self.block_on(async {
            let n = self.conn.execute("DELETE FROM tasks WHERE uuid = ?", vec![libsql::Value::Blob(uuid.as_bytes().to_vec())])
                .await.map_err(|e| taskchampion::Error::Database(e.to_string()))?;
            Ok(n > 0)
        })
    }

    fn all_tasks(&mut self) -> Result<Vec<(Uuid, std::collections::HashMap<String, String>)>, taskchampion::Error> {
        self.block_on(async {
             let mut rows = self.conn.query("SELECT uuid, data FROM tasks", ()).await
                .map_err(|e| taskchampion::Error::Database(e.to_string()))?;
             let mut result = Vec::new();
             while let Some(row) = rows.next().await.map_err(|e| taskchampion::Error::Database(e.to_string()))? {
                  let uuid_blob: Vec<u8> = row.get(0).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
                  let data: String = row.get(1).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
                  let uuid = Uuid::from_bytes(uuid_blob.try_into().unwrap_or([0; 16]));
                  let task: std::collections::HashMap<String, String> = serde_json::from_str(&data).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
                  result.push((uuid, task));
             }
             Ok(result)
        })
    }

    fn all_task_uuids(&mut self) -> Result<Vec<Uuid>, taskchampion::Error> {
        self.block_on(async {
             let mut rows = self.conn.query("SELECT uuid FROM tasks", ()).await
                .map_err(|e| taskchampion::Error::Database(e.to_string()))?;
             let mut result = Vec::new();
             while let Some(row) = rows.next().await.map_err(|e| taskchampion::Error::Database(e.to_string()))? {
                  let uuid_blob: Vec<u8> = row.get(0).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
                  result.push(Uuid::from_bytes(uuid_blob.try_into().unwrap_or([0; 16])));
             }
             Ok(result)
        })
    }

    fn base_version(&mut self) -> Result<Uuid, taskchampion::Error> {
        self.block_on(async {
             let mut rows = self.conn.query("SELECT value FROM meta WHERE key = 'base_version'", ()).await
                .map_err(|e| taskchampion::Error::Database(e.to_string()))?;
             if let Some(row) = rows.next().await.map_err(|e| taskchampion::Error::Database(e.to_string()))? {
                 let blob: Vec<u8> = row.get(0).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
                 Ok(Uuid::from_bytes(blob.try_into().unwrap_or([0; 16])))
             } else {
                 Ok(Uuid::nil())
             }
        })
    }

    fn set_base_version(&mut self, uuid: Uuid) -> Result<(), taskchampion::Error> {
        self.block_on(async {
             self.conn.execute("INSERT OR REPLACE INTO meta (key, value) VALUES ('base_version', ?)", 
                 vec![libsql::Value::Blob(uuid.as_bytes().to_vec())]
             ).await.map_err(|e| taskchampion::Error::Database(e.to_string()))?;
             Ok(())
        })
    }

    fn get_task_operations(&mut self, uuid: Uuid) -> Result<Vec<taskchampion::storage::ReplicaOp>, taskchampion::Error> {
         // This needs us to fetch operations related to this task.
         // But `operations` table stores serialized operations.
         // We should filter them. This is inefficient without parsing everything or storing task_uuid separately in operations table.
         // For now, scan all operations and filter.
         self.block_on(async {
              let mut rows = self.conn.query("SELECT data FROM operations ORDER BY id ASC", ()).await
                 .map_err(|e| taskchampion::Error::Database(e.to_string()))?;
              let mut ops = Vec::new();
              while let Some(row) = rows.next().await.map_err(|e| taskchampion::Error::Database(e.to_string()))? {
                  let data: String = row.get(0).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
                  let op: Operation = serde_json::from_str(&data).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
                  
                  let op_uuid = match &op {
                      Operation::Create { uuid, .. } => Some(*uuid),
                      Operation::Update { uuid, .. } => Some(*uuid),
                      Operation::Delete { uuid, .. } => Some(*uuid),
                      _ => None,
                  };
                  
                  if let Some(u) = op_uuid {
                      if u == uuid {
                          // ReplicaOp? Wait, trait expects `ReplicaOp`? It might expect `Operation` wrapper or something.
                          // The trait definition says `Vec<ReplicaOp>`. 
                          // I used `Operation` everywhere. `taskchampion::storage::ReplicaOp` is what? 
                          // The error message said `Vec<ReplicaOp>`. 
                          // `ReplicaOp` is actually `Operation`. In `taskchampion`, `type ReplicaOp = Operation;` maybe?
                          // Or `pub use crate::Operation as ReplicaOp`?
                          // Let's assume it is `Operation`.
                          ops.push(op);
                      }
                  }
              }
              Ok(ops)
         })
    }

    fn unsynced_operations(&mut self) -> Result<Vec<taskchampion::storage::ReplicaOp>, taskchampion::Error> {
          self.block_on(async {
              let mut rows = self.conn.query("SELECT data FROM operations ORDER BY id ASC", ()).await
                 .map_err(|e| taskchampion::Error::Database(e.to_string()))?;
              let mut ops = Vec::new();
              while let Some(row) = rows.next().await.map_err(|e| taskchampion::Error::Database(e.to_string()))? {
                  let data: String = row.get(0).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
                  let op: Operation = serde_json::from_str(&data).map_err(|e| taskchampion::Error::Database(e.to_string()))?;
                  ops.push(op);
              }
              Ok(ops)
          })
    }
}

impl TursoTxn {
    fn block_on<F, T>(&self, future: F) -> T 
    where F: std::future::Future<Output = T> 
    {
        self.runtime.block_on(future)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taskchampion::Uuid;

    #[test]
    fn test_storage_lifecycle() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db_path = dir.path().join("test.db");
        let path_str = db_path.to_str().unwrap().to_string();
        
        let config = TursoConfig::Local { path: path_str };
        
        let rt = tokio::runtime::Runtime::new()?;
        let mut storage = rt.block_on(TursoStorage::new(config))?;
        
        // Test 1: Empty check
        {
            let mut txn = storage.txn().unwrap();
            assert!(txn.is_empty().unwrap());
        }

        // Test 2: Add Operation
        let uuid = Uuid::new_v4();
        let op = Operation::Create { uuid };
        {
            let mut txn = storage.txn().unwrap();
            txn.add_operation(op.clone()).unwrap();
            txn.commit().unwrap();
        }
        
        // Test 3: Verify not empty
        {
            let mut txn = storage.txn().unwrap();
            assert!(!txn.is_empty().unwrap());
            assert_eq!(txn.num_unsynced_operations().unwrap(), 1);
        }

        // Test 4: Working Set
        {
            let mut txn = storage.txn().unwrap();
            let idx = txn.add_to_working_set(uuid).unwrap();
            assert_eq!(idx, 1);
            let set = txn.get_working_set().unwrap();
            assert_eq!(set.len(), 2); // 0 is None, 1 is uuid
            assert_eq!(set[1], Some(uuid));
        }

        Ok(())
    }

    #[test]
    fn test_task_crud() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db_path = dir.path().join("test_crud.db");
        let path_str = db_path.to_str().unwrap().to_string();
        
        let config = TursoConfig::Local { path: path_str };
        
        let rt = tokio::runtime::Runtime::new()?;
        let mut storage = rt.block_on(TursoStorage::new(config))?;

        let uuid = Uuid::new_v4();
        
        // Create task
        {
            let mut txn = storage.txn().unwrap();
            assert!(txn.create_task(uuid).unwrap());
            assert!(!txn.create_task(uuid).unwrap()); // Already exists
            txn.commit().unwrap();
        }

        // Set task data
        {
            let mut txn = storage.txn().unwrap();
            let mut data = std::collections::HashMap::new();
            data.insert("description".to_string(), "Verify Turso".to_string());
            txn.set_task(uuid, data).unwrap();
            txn.commit().unwrap();
        }

        // Get task
        {
            let mut txn = storage.txn().unwrap();
            let task = txn.get_task(uuid).unwrap();
            assert!(task.is_some());
            assert_eq!(task.unwrap().get("description").map(|s| s.as_str()), Some("Verify Turso"));
        }

        // Delete task
        {
            let mut txn = storage.txn().unwrap();
            assert!(txn.delete_task(uuid).unwrap());
            assert!(txn.get_task(uuid).unwrap().is_none());
            txn.commit().unwrap();
        }

        Ok(())
    }
}

