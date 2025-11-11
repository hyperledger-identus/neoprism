# SQLite backend notes

- **When to pick SQLite**: ideal for local development, CI smoke tests, and air-gapped demos. For production workloads or multi-writer setups stick with PostgreSQL.
- **Limitations**: single-writer, WAL must remain enabled (we already enforce this in the pool setup), and long-running readers can block checkpointing. Monitor file size and consider VACUUM if needed.
- **Default location**: `~/Library/Application Support/NeoPRISM/<network>/neoprism.db` on macOS, `$XDG_DATA_HOME/NeoPRISM/<network>/neoprism.db` on Linux, `%APPDATA%\NeoPRISM\<network>\neoprism.db` on Windows. Override via `--db-url sqlite:///custom/path.db`.
- **Backups**: use filesystem snapshots, the `just db-backup sqlite <file>` helper, or `sqlite3 neoprism.db ".backup backup.db"`. Snapshots produced via `neoprism-node db backup` can be restored into either backend.
