-- The support system; see docs/SUPPORT.MD. Tickets hold messages; FAQs stand alone.

CREATE TYPE ticket_category AS ENUM ('OTHER', 'USER_REPORT', 'SUGGESTION', 'BUG_REPORT', 'USER_ISSUE');
CREATE TYPE support_status AS ENUM ('OPEN', 'AWAITING_USER', 'AWAITING_ADMIN', 'RESOLVED', 'CLOSED');
CREATE TYPE faq_category AS ENUM ('GENERAL', 'SUPPORT', 'STORE', 'FORUM', 'USER', 'COMMUNITY');

CREATE TABLE support_tickets (
    id                  BIGSERIAL       PRIMARY KEY,
    title               TEXT            NOT NULL CHECK (btrim(title) <> ''),
    -- Short description for glancing at a list of tickets.
    subject             TEXT            NOT NULL CHECK (char_length(subject) <= 256),
    user_id             BIGINT          NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Other users the ticket involves, chosen by the user when creating it.
    associated_user_ids BIGINT[]        NOT NULL DEFAULT '{}',
    -- The ticket's first message. Created in the same transaction as the ticket, so the
    -- foreign key (added below) is deferred.
    initial_message_id  BIGINT          NOT NULL,
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT now(),
    -- Bumped by messages and status changes; drives the 7-day auto-close.
    last_updated        TIMESTAMPTZ     NOT NULL DEFAULT now(),
    category            ticket_category NOT NULL DEFAULT 'OTHER',
    status              support_status  NOT NULL DEFAULT 'OPEN',
    resolved_at         TIMESTAMPTZ,
    closed_at           TIMESTAMPTZ,
    -- Who resolved or closed it; NULL when closed automatically.
    closed_by           BIGINT          REFERENCES users (id) ON DELETE SET NULL,
    -- Support team members who have responded to or changed the ticket.
    responders          BIGINT[]        NOT NULL DEFAULT '{}'
);

CREATE INDEX support_tickets_user_idx ON support_tickets (user_id, last_updated DESC);
CREATE INDEX support_tickets_status_idx ON support_tickets (status, last_updated DESC);

CREATE TABLE support_messages (
    id               BIGSERIAL   PRIMARY KEY,
    author_id        BIGINT      NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    ticket_id        BIGINT      NOT NULL REFERENCES support_tickets (id) ON DELETE CASCADE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_updated     TIMESTAMPTZ NOT NULL DEFAULT now(),
    content          TEXT        NOT NULL CHECK (btrim(content) <> ''),
    -- Messages can be edited once; this keeps what they said before.
    previous_content TEXT,
    -- Only the support team can see internal messages.
    is_internal      BOOLEAN     NOT NULL DEFAULT false
);

CREATE INDEX support_messages_ticket_idx ON support_messages (ticket_id, created_at);

ALTER TABLE support_tickets
    ADD CONSTRAINT support_tickets_initial_message_fk
    FOREIGN KEY (initial_message_id) REFERENCES support_messages (id)
    ON DELETE CASCADE
    DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE faqs (
    id           BIGSERIAL    PRIMARY KEY,
    title        TEXT         NOT NULL CHECK (btrim(title) <> ''),
    subject      TEXT         NOT NULL CHECK (char_length(subject) <= 256),
    category     faq_category NOT NULL DEFAULT 'GENERAL',
    -- Markdown.
    description  TEXT         NOT NULL CHECK (btrim(description) <> ''),
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT now(),
    last_updated TIMESTAMPTZ  NOT NULL DEFAULT now(),
    created_by   BIGINT       NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Everyone who has edited it, in order of their first edit.
    editors      BIGINT[]     NOT NULL DEFAULT '{}'
);

CREATE INDEX faqs_category_idx ON faqs (category, last_updated DESC);

-- The default `admin` group has every permission, but existing groups are never
-- overwritten on startup, so grant the new Support Team bit (0x1000) here.
UPDATE usergroups SET permissions = permissions | 4096, last_updated = now()
WHERE lower(name) = 'admin' AND permissions & 4096 = 0;
