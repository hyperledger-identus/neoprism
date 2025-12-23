PRAGMA foreign_keys = ON;

-- Delete all existing records from indexed tables first
DELETE FROM indexed_vdr_operation;
DELETE FROM indexed_ssi_operation;

-- Delete all existing records from raw_operation
DELETE FROM raw_operation;

-- Reset cursor so indexer starts from beginning
DELETE FROM dlt_cursor;

-- Add tx_hash column to raw_operation table
ALTER TABLE raw_operation ADD COLUMN tx_hash BLOB NOT NULL;

-- Recreate views to include tx_hash
DROP VIEW IF EXISTS did_stats;
DROP VIEW IF EXISTS raw_operation_by_did;

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
    ro.tx_hash,
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

-- Add index on tx_hash column for efficient transaction lookup
CREATE INDEX idx_raw_operation_tx_hash ON raw_operation(tx_hash);
