-- Add up migration script here
ALTER TABLE dispatch_content
    ALTER COLUMN created_at TYPE timestamptz;

ALTER TABLE dispatch_queue
    ALTER COLUMN created_at TYPE timestamptz,
    ALTER COLUMN modified_at TYPE timestamptz;

ALTER TABLE dispatches
    ALTER COLUMN created_at TYPE timestamptz,
    ALTER COLUMN updated_at TYPE timestamptz;

ALTER TABLE rmbpost_queue
    ALTER COLUMN created_at TYPE timestamptz,
    ALTER COLUMN updated_at TYPE timestamptz;

ALTER TABLE users
    ALTER COLUMN created_at TYPE timestamptz;