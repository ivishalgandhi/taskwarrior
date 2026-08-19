# Turso Remote only

Taskwarrior’s Turso path opens a cloud libsql database with URL + auth token only. We rejected EmbeddedReplica / `turso.file` after that mode caused repeated sync and connection thrash; bringing it back would reopen a second sync model behind the same storage adapter. Callers that set `turso.file` must fail at open with a clear error.
