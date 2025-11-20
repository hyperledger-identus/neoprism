DROP TABLE IF EXISTS seaql_migrations;

CREATE TABLE IF NOT EXISTS dlt_cursor (
    slot int8 NOT NULL,
    block_hash bytea NOT NULL,
    CONSTRAINT dlt_cursor_pkey PRIMARY KEY (slot, block_hash)
);

CREATE TABLE IF NOT EXISTS raw_operation (
    did bytea NOT NULL,
    signed_operation_data bytea NOT NULL,
    slot int8 NOT NULL,
    block_number int8 NOT NULL,
    cbt timestamptz NOT NULL,
    absn int4 NOT NULL,
    osn int4 NOT NULL,
    CONSTRAINT raw_operation_pkey PRIMARY KEY (did, block_number, absn, osn)
);

-- migrate primary key for raw_operation
ALTER TABLE raw_operation DROP CONSTRAINT raw_operation_pkey;

ALTER TABLE raw_operation
ADD COLUMN id uuid DEFAULT gen_random_uuid();

ALTER TABLE raw_operation
ADD CONSTRAINT raw_operation_pkey PRIMARY KEY (id);

ALTER TABLE raw_operation
ADD CONSTRAINT raw_operation_abs_order UNIQUE (did, block_number, absn, osn);

-- migrate primary key for dlt_cursor
ALTER TABLE dlt_cursor DROP CONSTRAINT dlt_cursor_pkey;

ALTER TABLE dlt_cursor
ADD COLUMN id uuid DEFAULT gen_random_uuid();

ALTER TABLE dlt_cursor
ADD CONSTRAINT dlt_cursor_pkey PRIMARY KEY (id);

ALTER TABLE dlt_cursor
ADD CONSTRAINT dlt_cursor_abs_order UNIQUE (slot, block_hash);

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
FROM raw_operation
GROUP BY 1;
