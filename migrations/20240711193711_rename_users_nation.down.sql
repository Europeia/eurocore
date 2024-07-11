-- Add down migration script here
ALTER TABLE users RENAME COLUMN username TO nation;