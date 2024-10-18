-- Add up migration script here
ALTER TABLE dispatch_queue
    RENAME COLUMN updated_at TO modified_at;