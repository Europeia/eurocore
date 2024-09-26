-- Add up migration script here
ALTER TABLE dispatch_queue
    ADD COLUMN dispatch_id INT;