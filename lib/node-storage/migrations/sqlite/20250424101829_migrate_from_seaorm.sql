PRAGMA foreign_keys = ON;

DROP VIEW IF EXISTS did_stats;
DROP VIEW IF EXISTS raw_operation_by_did;
DROP TABLE IF EXISTS indexed_vdr_operation;
DROP TABLE IF EXISTS indexed_ssi_operation;
DROP TABLE IF EXISTS raw_operation;
DROP TABLE IF EXISTS dlt_cursor;

CREATE TABLE IF NOT EXISTS dlt_cursor (
    id BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    slot INTEGER NOT NULL,
    block_hash BLOB NOT NULL,
    UNIQUE(slot, block_hash)
);

CREATE TABLE IF NOT EXISTS raw_operation (
    id BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    signed_operation_data BLOB NOT NULL,
    slot INTEGER NOT NULL,
    block_number INTEGER NOT NULL,
    cbt TEXT NOT NULL,
    absn INTEGER NOT NULL,
    osn INTEGER NOT NULL,
    is_indexed INTEGER NOT NULL DEFAULT 0,
    UNIQUE(block_number, absn, osn)
);

CREATE TABLE IF NOT EXISTS indexed_ssi_operation (
    id BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    raw_operation_id BLOB NOT NULL UNIQUE,
    did BLOB NOT NULL,
    indexed_at TEXT NOT NULL,
    FOREIGN KEY (raw_operation_id) REFERENCES raw_operation(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS indexed_vdr_operation (
    id BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    raw_operation_id BLOB NOT NULL UNIQUE,
    operation_hash BLOB NOT NULL,
    init_operation_hash BLOB NOT NULL,
    prev_operation_hash BLOB,
    did BLOB NOT NULL,
    indexed_at TEXT NOT NULL,
    FOREIGN KEY (raw_operation_id) REFERENCES raw_operation(id) ON DELETE CASCADE
);

CREATE VIEW raw_operation_by_did AS
WITH unioned AS (
    SELECT did, raw_operation_id FROM indexed_ssi_operation
    UNION
    SELECT did, raw_operation_id FROM indexed_vdr_operation
)
SELECT
    ro.id,
    ro.signed_operation_data,
    ro.slot,
    ro.block_number,
    ro.cbt,
    ro.absn,
    ro.osn,
    ro.is_indexed,
    u.did
FROM unioned AS u
LEFT JOIN raw_operation AS ro ON u.raw_operation_id = ro.id;

CREATE VIEW did_stats AS
SELECT
    did,
    COUNT(*) AS operation_count,
    MAX(block_number) AS last_block,
    MAX(slot) AS last_slot,
    MAX(cbt) AS last_cbt,
    MIN(block_number) AS first_block,
    MIN(slot) AS first_slot,
    MIN(cbt) AS first_cbt
FROM raw_operation_by_did
GROUP BY 1;
