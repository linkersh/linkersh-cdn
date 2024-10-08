-- Add up migration script here

DELETE FROM cdn_objects;

ALTER TABLE cdn_objects
ADD COLUMN sha256_hash CHAR(64) NOT NULL;