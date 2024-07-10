-- Add up migration script here
CREATE TABLE IF NOT EXISTS user_permissions (
  id SERIAL PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  permission_id INTEGER NOT NULL REFERENCES permissions (id) ON DELETE CASCADE
);