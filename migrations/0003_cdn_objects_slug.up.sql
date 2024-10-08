-- Add up migration script here
ALTER TABLE cdn_objects
ADD COLUMN slug VARCHAR(8);

ALTER TABLE cdn_objects
ADD COLUMN is_public BOOLEAN NOT NULL DEFAULT false;
