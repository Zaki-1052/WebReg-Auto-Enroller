-- Create sections table
CREATE TABLE IF NOT EXISTS sections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    course_id UUID NOT NULL REFERENCES courses(id) ON DELETE CASCADE,
    lecture VARCHAR(20) NOT NULL,
    discussions JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index on course_id for fast lookups
CREATE INDEX idx_sections_course_id ON sections(course_id);
