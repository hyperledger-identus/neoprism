-- Delete all existing records from indexed tables first
DELETE FROM indexed_vdr_operation;
DELETE FROM indexed_ssi_operation;

-- Delete all existing records from raw_operation
DELETE FROM raw_operation;

-- Reset cursor so indexer starts from beginning
DELETE FROM dlt_cursor;

-- Add tx_hash and operation_id columns to raw_operation table
ALTER TABLE raw_operation
ADD COLUMN tx_hash BYTEA NOT NULL,
ADD COLUMN operation_id BYTEA NOT NULL;

-- Recreate views to include tx_hash
DROP VIEW IF EXISTS did_stats;
DROP VIEW IF EXISTS raw_operation_by_did;

CREATE VIEW raw_operation_by_did AS
WITH unioned AS (
    SELECT
        did,
        raw_operation_id
    FROM indexed_ssi_operation
    UNION
    SELECT
        did,
        raw_operation_id
    FROM indexed_vdr_operation
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
    ro.operation_id,
    ro.is_indexed,
    u.did
FROM unioned AS u LEFT JOIN raw_operation AS ro ON u.raw_operation_id = ro.id;

CREATE VIEW did_stats AS
SELECT
    did,
    count(*) AS operation_count,
    max(block_number) AS last_block,
    max(slot) AS last_slot,
    max(cbt) AS last_cbt,
    min(block_number) AS first_block,
    min(slot) AS first_slot,
    min(cbt) AS first_cbt
FROM raw_operation_by_did
GROUP BY 1;

-- Add indexes for efficient transaction and operation lookups
CREATE INDEX idx_raw_operation_tx_hash ON raw_operation (tx_hash);
CREATE INDEX idx_raw_operation_operation_id ON raw_operation (operation_id);
