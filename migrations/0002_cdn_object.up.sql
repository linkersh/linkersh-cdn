-- Add up migration script here
CREATE TABLE cdn_objects (
    id              UUID NOT NULL,
    user_id UUID    NOT NULL REFERENCES users(id),
    uploaded_at     TIMESTAMP NOT NULL DEFAULT NOW(),
    content_type    VARCHAR(64) NOT NULL,
    content_size    BIGINT NOT NULL,
    file_name       VARCHAR(128) NOT NULL,

    PRIMARY KEY (id)
);