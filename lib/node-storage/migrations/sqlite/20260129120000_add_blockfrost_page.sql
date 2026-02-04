-- Add blockfrost_page column to dlt_cursor table for pagination tracking
ALTER TABLE dlt_cursor ADD COLUMN blockfrost_page INTEGER;
