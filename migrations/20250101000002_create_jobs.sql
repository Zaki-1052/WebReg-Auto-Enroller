-- Create monitoring jobs table
CREATE TABLE IF NOT EXISTS jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    term VARCHAR(10) NOT NULL,
    polling_interval INTEGER NOT NULL DEFAULT 30,
    cookie_encrypted TEXT NOT NULL,
    encryption_nonce TEXT NOT NULL,
    seat_threshold INTEGER NOT NULL DEFAULT 0,
    monitoring_mode VARCHAR(20) NOT NULL DEFAULT 'Include',
    is_active BOOLEAN NOT NULL DEFAULT false,
    is_connected BOOLEAN NOT NULL DEFAULT false,
    last_check_time TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index on user_id for fast lookups
CREATE INDEX idx_jobs_user_id ON jobs(user_id);

-- Index on is_active for filtering active jobs
CREATE INDEX idx_jobs_active ON jobs(is_active);
