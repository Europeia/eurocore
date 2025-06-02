-- Add up migration script here
ALTER TABLE user_permissions
    ADD CONSTRAINT user_permissions_user_id_permission_id_key
        UNIQUE (user_id, permission_id);
