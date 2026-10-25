CREATE TABLE fixture_repositories (
    repo_id text PRIMARY KEY,
    authorized_actor text NOT NULL,
    generation bigint NOT NULL DEFAULT 0 CHECK (generation >= 0),
    recovery_epoch bigint NOT NULL DEFAULT 1 CHECK (recovery_epoch > 0),
    fence_token bigint NOT NULL DEFAULT 1 CHECK (fence_token > 0),
    candidate_id text
);
CREATE TABLE fixture_candidates (
    candidate_id text PRIMARY KEY,
    payload bytea NOT NULL CHECK (octet_length(payload) <= 65536)
);
CREATE TABLE fixture_outcomes (
    operation_id text PRIMARY KEY,
    repo_id text NOT NULL REFERENCES fixture_repositories,
    actor_id text NOT NULL,
    request_key text NOT NULL,
    input_fingerprint text NOT NULL,
    candidate_id text NOT NULL REFERENCES fixture_candidates,
    generation bigint NOT NULL CHECK (generation > 0),
    recovery_epoch bigint NOT NULL,
    fence_token bigint NOT NULL,
    UNIQUE (repo_id, actor_id, request_key),
    UNIQUE (repo_id, generation)
);
CREATE TABLE fixture_outbox (
    operation_id text PRIMARY KEY REFERENCES fixture_outcomes,
    repo_id text NOT NULL REFERENCES fixture_repositories,
    candidate_id text NOT NULL REFERENCES fixture_candidates,
    generation bigint NOT NULL
);
