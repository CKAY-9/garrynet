-- Clearing the staging keys of recently completed S3 uploads (docs/STORAGE.md), every 10
-- minutes, by when the upload started.
CREATE INDEX files_s3_ready_idx ON files (created_at) WHERE backend = 's3' AND status = 'READY';
