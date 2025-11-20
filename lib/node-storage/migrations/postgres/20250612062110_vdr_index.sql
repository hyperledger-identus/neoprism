-- migrate raw_operation to not be indexed by did
DROP VIEW did_stats RESTRICT;

ALTER TABLE raw_operation
DROP COLUMN did;

ALTER TABLE raw_operation
ADD CONSTRAINT raw_operation_abs_order UNIQUE (block_number, absn, osn);

ALTER TABLE raw_operation
ADD COLUMN is_indexed BOOLEAN;

UPDATE raw_operation SET is_indexed = false
WHERE is_indexed IS null;

ALTER TABLE raw_operation
ALTER COLUMN is_indexed SET NOT NULL;

-- add indexing tables
CREATE TABLE IF NOT EXISTS indexed_ssi_operation (
    id UUID DEFAULT gen_random_uuid(),
    raw_operation_id UUID NOT NULL UNIQUE,
    did BYTEA NOT NULL,
    indexed_at TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (raw_operation_id) REFERENCES raw_operation (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS indexed_vdr_operation (
    id UUID DEFAULT gen_random_uuid(),
    raw_operation_id UUID NOT NULL UNIQUE,
    operation_hash BYTEA NOT NULL,
    init_operation_hash BYTEA NOT NULL,
    prev_operation_hash BYTEA,
    did BYTEA NOT NULL,
    indexed_at TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (raw_operation_id) REFERENCES raw_operation (id) ON DELETE CASCADE
);

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
