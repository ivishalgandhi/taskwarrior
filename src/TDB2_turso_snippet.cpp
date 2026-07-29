void TDB2::open_replica_turso(const std::string& config_json) {
  _replica = tc::new_replica_with_turso(config_json);
}
