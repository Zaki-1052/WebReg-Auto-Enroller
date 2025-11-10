-- Create enrollment stats table
CREATE TABLE IF NOT EXISTS enrollment_stats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    total_checks INTEGER NOT NULL DEFAULT 0,
    openings_found INTEGER NOT NULL DEFAULT 0,
    enrollment_attempts INTEGER NOT NULL DEFAULT 0,
    successful_enrollments INTEGER NOT NULL DEFAULT 0,
    errors INTEGER NOT NULL DEFAULT 0,
    section_failures JSONB NOT NULL DEFAULT '{}',
    start_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index on job_id for fast lookups
CREATE INDEX idx_stats_job_id ON enrollment_stats(job_id);

-- Unique constraint to ensure one stats record per job
CREATE UNIQUE INDEX idx_stats_unique_job ON enrollment_stats(job_id);
