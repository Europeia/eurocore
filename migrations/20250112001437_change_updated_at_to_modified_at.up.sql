-- Add up migration script here
ALTER TABLE dispatches
    RENAME COLUMN updated_at TO modified_at;

ALTER TABLE rmbpost_queue
    RENAME COLUMN updated_at TO modified_at;