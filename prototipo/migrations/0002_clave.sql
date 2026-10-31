ALTER TABLE versions ADD COLUMN IF NOT EXISTS clave TEXT NOT NULL DEFAULT '';
ALTER TABLE versions ALTER COLUMN published_at TYPE TEXT USING published_at::text;
ALTER TABLE versions ALTER COLUMN published_at SET DEFAULT now()::text;
CREATE INDEX IF NOT EXISTS versions_clave_idx ON versions(clave);