set working-directory := '../..'

# PostgreSQL configuration

db_port := "5432"
db_user := "postgres"
db_pass := "postgres"
db_name := "postgres"

# Embedded SQLite defaults

sqlite_db_path := "data/sqlite/neoprism-dev.sqlite"
sqlite_db_url := "sqlite://data/sqlite/neoprism-dev.sqlite"

# ============================================================================
# PostgreSQL Database
# ============================================================================

# Start local PostgreSQL database in Docker
postgres-up:
    docker run \
      -d --rm \
      --name prism-db \
      -e POSTGRES_DB={{ db_name }} \
      -e POSTGRES_USER={{ db_user }} \
      -e POSTGRES_PASSWORD={{ db_pass }} \
      -p {{ db_port }}:5432 postgres:16

# Stop local PostgreSQL database
postgres-down:
    docker stop prism-db

# Dump PostgreSQL database to postgres.dump file
postgres-dump:
    export PGPASSWORD={{ db_pass }} && \
        pg_dump -h localhost -p {{ db_port }} -U {{ db_user }} -w -d {{ db_name }} -Fc > postgres.dump && \
        echo "Database dumped to postgres.dump"

# Restore PostgreSQL database from postgres.dump file
postgres-restore:
    export PGPASSWORD={{ db_pass }} && \
        pg_restore -h localhost -p {{ db_port }} -U {{ db_user }} -w -d {{ db_name }} postgres.dump && \
        echo "Database restored from postgres.dump"

# ============================================================================
# SQLite Database
# ============================================================================

# Initialize the embedded SQLite database
sqlite-init:
    mkdir -p "$(dirname {{ sqlite_db_path }})"
    touch {{ sqlite_db_path }}
    echo "SQLite database initialized at {{ sqlite_db_path }}"

# Remove the embedded SQLite database file
sqlite-clean:
    rm -f {{ sqlite_db_path }}
    echo "Removed SQLite database at {{ sqlite_db_path }}"
