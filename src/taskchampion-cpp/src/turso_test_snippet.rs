
#[cfg(test)]
mod tests {
    use super::*;
    use taskchampion::Uuid;

    #[test]
    fn test_storage_lifecycle() -> Result<()> {
        let runtime = std::sync::Arc::new(tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?);

        let dir = tempfile::tempdir()?;
        let db_path = dir.path().join("test.db");
        let path_str = db_path.to_str().unwrap().to_string();
        
        let config = TursoConfig::Local { path: path_str };
        let mut storage = runtime.block_on(async {
            TursoStorage::new(config, runtime.clone()).await
        })?;
        
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
}
