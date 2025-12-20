
    #[tokio::test]
    #[ignore] // Only run when TURSO_URL and TURSO_TOKEN are set
    async fn test_real_turso_connectivity() -> Result<()> {
        // Get credentials from environment variables
        let url = std::env::var("TURSO_URL")
            .expect("Set TURSO_URL environment variable to run this test");
        let token = std::env::var("TURSO_TOKEN")
            .expect("Set TURSO_TOKEN environment variable to run this test");
        
        let config = TursoConfig::Remote { url, token };
        let runtime = std::sync::Arc::new(tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?);
        let mut storage = TursoStorage::new(config, runtime).await?;
        
        // Try to read generic metadata or just check if txn creation works
        {
             let mut txn = storage.txn()?;
             // If we can create a txn and check is_empty, we connected successfully
             let _ = txn.is_empty()?;
        }
        Ok(())
    }

