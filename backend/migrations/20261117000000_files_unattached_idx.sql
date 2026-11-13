-- Sweeping user files that were uploaded but never attached to anything (docs/STORAGE.md).
CREATE INDEX files_unattached_idx ON files (completed_at)
    WHERE owner_id > 0 AND status = 'READY'
      AND forum_post_id IS NULL AND support_message_id IS NULL AND private_message_id IS NULL;
