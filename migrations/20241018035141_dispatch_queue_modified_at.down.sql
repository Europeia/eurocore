-- Add down migration script here
ALTER TABLE dispatch_queue
    RENAME COLUMN modified_at TO updated_at;