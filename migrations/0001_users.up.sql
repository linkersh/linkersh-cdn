-- Add up migration script here
CREATE TABLE users (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    github_id BIGINT NOT NULL,
    username TEXT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE user_secret_codes (
    user_id UUID NOT NULL REFERENCES users(id),
    code VARCHAR(32) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),

    PRIMARY KEY (user_id, code)
);
