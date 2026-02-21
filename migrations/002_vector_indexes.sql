-- Migration 002: Vector indexes
-- Run AFTER initial bulk data load, not before.
-- IVFFlat requires data to exist for optimal centroid selection.
-- See ARCHITECTURE.md §1.4 (pgvector usage)

-- Chunks embedding index (primary search index)
-- lists = sqrt(expected_row_count); start with 100, tune later
CREATE INDEX IF NOT EXISTS idx_chunks_embedding
    ON paper_chunks USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);

-- Entity embedding index (for semantic similarity queries)
CREATE INDEX IF NOT EXISTS idx_entities_embedding
    ON entities USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 50);

INSERT INTO schema_migrations (version, checksum)
VALUES ('002_vector_indexes', 'v0.1.0')
ON CONFLICT (version) DO NOTHING;
