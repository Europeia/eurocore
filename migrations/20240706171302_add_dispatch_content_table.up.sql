-- Add up migration script here
CREATE TABLE IF NOT EXISTS dispatch_content (
    id SERIAL PRIMARY KEY,
    dispatch_id INTEGER NOT NULL REFERENCES dispatches (id) ON DELETE CASCADE,
    category SMALLINT NOT NULL,
    subcategory SMALLINT NOT NULL,
    title VARCHAR NOT NULL,
    text VARCHAR NOT NULL,
    created_by VARCHAR NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);