-- Add down migration script here
ALTER TABLE user_permissions
    DROP CONSTRAINT user_permissions_user_id_permission_id_key;