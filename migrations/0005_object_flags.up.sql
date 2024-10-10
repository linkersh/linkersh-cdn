-- Add up migration script here
ALTER TABLE cdn_objects
ADD COLUMN flags BIGINT NOT NULL DEFAULT 0;