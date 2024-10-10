-- Add down migration script here
ALTER TABLE cdn_objects
DROP COLUMN flags;