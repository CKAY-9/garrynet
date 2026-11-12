-- Notifications for private messages (docs/NOTIFICATIONS.md). Its own migration: a value
-- added to an enum can't be used in the transaction that adds it.

ALTER TYPE notification_type ADD VALUE 'MESSAGE' BEFORE 'ACCOUNT';
