-- Add down migration script here
ALTER TABLE rmbpost_queue
    ADD COLUMN payload JSONB;