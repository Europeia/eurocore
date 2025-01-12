-- Add down migration script here
ALTER TABLE dispatches
    RENAME COLUMN modified_at TO updated_at;

ALTER TABLE rmbpost_queue
    RENAME COLUMN modified_at TO updated_at;