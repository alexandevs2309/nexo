CREATE TABLE users (
    id UUID PRIMARY KEY,
    username TEXT NOT NULL UNIQUE CHECK (username ~ '^[A-Za-z0-9_-]{1,64}$'),
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('admin', 'reader', 'publisher')),
    disabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE access_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id),
    token_hash BYTEA NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX access_tokens_user_id_idx ON access_tokens(user_id);

CREATE TABLE namespaces (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL UNIQUE CHECK (name ~ '^[A-Za-z0-9_-]{1,64}$'),
    owner_id UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE namespace_members (
    namespace_id UUID NOT NULL REFERENCES namespaces(id),
    user_id UUID NOT NULL REFERENCES users(id),
    role TEXT NOT NULL CHECK (role IN ('owner', 'publisher', 'reader')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (namespace_id, user_id)
);

CREATE TABLE packages (
    id UUID PRIMARY KEY,
    namespace TEXT NOT NULL,
    name TEXT NOT NULL CHECK (name ~ '^[A-Za-z0-9_-]{1,100}$'),
    description TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(namespace, name)
);

CREATE TABLE versions (
    id UUID PRIMARY KEY,
    package_id UUID NOT NULL REFERENCES packages(id),
    version TEXT NOT NULL,
    sha256 TEXT NOT NULL CHECK (sha256 ~ '^[a-f0-9]{64}$'),
    registry_id TEXT NOT NULL,
    key_id TEXT NOT NULL,
    signature TEXT NOT NULL,
    nonce TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    artifact_path TEXT NOT NULL UNIQUE,
    artifact_size BIGINT NOT NULL DEFAULT 1 CHECK (artifact_size > 0),
    dependencies JSONB NOT NULL DEFAULT '{}'::jsonb,
    published_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_by UUID REFERENCES users(id),
    yanked BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE(package_id, version)
);
CREATE INDEX versions_package_published_idx ON versions(package_id, published_at DESC);

CREATE TABLE keys (
    id UUID PRIMARY KEY,
    registry_id TEXT NOT NULL,
    key_id TEXT NOT NULL,
    public_key BYTEA NOT NULL,
    valid_from TIMESTAMPTZ NOT NULL,
    valid_until TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    revocation_reason TEXT,
    UNIQUE(registry_id, key_id)
);

CREATE TABLE audit (
    id UUID PRIMARY KEY,
    action TEXT NOT NULL,
    actor UUID REFERENCES users(id),
    detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    request_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX audit_created_at_idx ON audit(created_at DESC);
