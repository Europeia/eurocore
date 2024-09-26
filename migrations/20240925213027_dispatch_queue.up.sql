-- Add up migration script here
CREATE TABLE dispatch_queue
(
    id         SERIAL PRIMARY KEY,
    type       VARCHAR(255) NOT NULL,
    payload    JSONB        NOT NULL,
    status     VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);