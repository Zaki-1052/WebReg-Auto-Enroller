-- Create notification settings table
CREATE TABLE IF NOT EXISTS notification_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    gmail_address VARCHAR(255),
    gmail_app_password_encrypted TEXT,
    gmail_encryption_nonce TEXT,
    email_recipients JSONB NOT NULL DEFAULT '[]',
    discord_webhook_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index on user_id for fast lookups
CREATE INDEX idx_notifications_user_id ON notification_settings(user_id);

-- Unique constraint to ensure one notification setting per user
CREATE UNIQUE INDEX idx_notifications_unique_user ON notification_settings(user_id);
