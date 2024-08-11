-- Add up migration script here
ALTER TABLE dispatches ADD COLUMN nation VARCHAR(255);
UPDATE dispatches SET nation = 'upc_the_founder';
ALTER TABLE dispatches ALTER COLUMN nation SET NOT NULL;