-- Add up migration script here
ALTER TABLE rmbpost_queue
    ADD nation  TEXT,
    ADD region  TEXT,
    ADD content TEXT;
