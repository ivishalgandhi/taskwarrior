# Taskwarrior (Turso fork)

Domain language for this fork’s storage and sync choices, especially the Turso backend.

## Language

**Replica**:
The TaskChampion task database a Taskwarrior process opens for the session — either on-disk SQLite or Turso Remote.
_Avoid_: database connection, backend handle, storage instance (when you mean the opened Replica)

**Turso Remote**:
The only supported Turso mode: tasks live in a cloud libsql database addressed by a URL and auth token; every operation needs the network.
_Avoid_: EmbeddedReplica, turso.file, local Turso replica, hybrid sync

**Turso credentials**:
The pair of `turso.url` and `turso.token` required to open a Turso Remote Replica. Presence of `turso.url` selects Turso over on-disk storage; both must be non-empty, and `turso.file` must not be set.
_Avoid_: TursoConfig JSON, config blob (as the cross-language open contract)

**On-disk storage**:
The default TaskChampion SQLite Replica under the Taskwarrior data directory when Turso is not configured.
_Avoid_: local mode (ambiguous with deleted Turso Local/Embedded paths)
