-- Add down migration script here
ALTER TABLE dispatch_content
    ALTER COLUMN created_at TYPE timestamp;

ALTER TABLE dispatch_queue
    ALTER COLUMN created_at TYPE timestamp,
    ALTER COLUMN modified_at TYPE timestamp;

ALTER TABLE dispatches
    ALTER COLUMN created_at TYPE timestamp,
    ALTER COLUMN updated_at TYPE timestamp;

ALTER TABLE rmbpost_queue
    ALTER COLUMN created_at TYPE timestamp,
    ALTER COLUMN updated_at TYPE timestamp;

ALTER TABLE users
    ALTER COLUMN created_at TYPE timestamp;