-- Add down migration script here
ALTER TABLE cdn_objects
DROP COLUMN slug;

ALTER TABLE cdn_objects
DROP COLUMN is_public;