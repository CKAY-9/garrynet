-- The community's legal documents (docs/LEGAL.md), as Markdown. The backend inserts its
-- default templates (backend/legal/*.md) on startup when a document is missing; after
-- that, admins edit them and the backend never overwrites them.
CREATE TABLE legal_documents (
    slug         TEXT        PRIMARY KEY CHECK (slug IN ('terms', 'privacy', 'refunds')),
    title        TEXT        NOT NULL CHECK (btrim(title) <> ''),
    body         TEXT        NOT NULL,
    -- Null while the document is still the untouched default template.
    last_updated TIMESTAMPTZ,
    updated_by   BIGINT      REFERENCES users (id) ON DELETE SET NULL
);
