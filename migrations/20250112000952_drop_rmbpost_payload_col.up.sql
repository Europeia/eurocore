-- Add up migration script here
ALTER TABLE rmbpost_queue
    DROP COLUMN payload;