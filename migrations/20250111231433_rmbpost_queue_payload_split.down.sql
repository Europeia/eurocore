-- Add down migration script here
ALTER TABLE rmbpost_queue
    DROP COLUMN nation,
    DROP COLUMN region,
    DROP COLUMN content;