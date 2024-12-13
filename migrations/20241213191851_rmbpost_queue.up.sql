-- Add up migration script here
CREATE TABLE rmbpost_queue 
(
    id SERIAL PRIMARY KEY,
    rmbpost_id INTEGER,
    text VARCHAR NOT NULL,
    status VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    error TEXT,
);
