-- Add down migration script here
ALTER TABLE dispatch_queue
    DROP COLUMN error;