-- V12__performance_optimizations.sql
-- Performance optimizations for oncology workloads

-- Indexes for paper search and filtering
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_papers_pub_date ON papers (pub_date);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_papers_journal ON papers (journal);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_papers_open_access ON papers (open_access);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_papers_source ON papers (source);

-- Indexes for paper chunks (vector search optimization)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_paper_chunks_paper_id ON paper_chunks (paper_id);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_paper_chunks_section_type ON paper_chunks (section_type);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_paper_chunks_token_count ON paper_chunks (token_count);

-- Indexes for entities (gene/disease/chemical search)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entities_paper_id ON entities (paper_id);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entities_entity_type ON entities (entity_type);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entities_normalized_id ON entities (normalized_id) WHERE normalized_id IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entities_score ON entities (score);

-- Indexes for knowledge graph facts
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_kg_facts_paper_id ON kg_facts (paper_id);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_kg_facts_subject_id ON kg_facts (subject_id);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_kg_facts_object_id ON kg_facts (object_id);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_kg_facts_predicate ON kg_facts (predicate);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_kg_facts_confidence ON kg_facts (confidence);

-- Indexes for target scores (ranking and prioritization)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_target_scores_gene_entity_id ON target_scores (gene_entity_id);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_target_scores_cancer_entity_id ON target_scores (cancer_entity_id);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_target_scores_composite_score ON target_scores (composite_score DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_target_scores_is_current ON target_scores (is_current) WHERE is_current = true;

-- Composite indexes for common query patterns
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_papers_pub_date_journal ON papers (pub_date DESC, journal) WHERE journal IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entities_type_score ON entities (entity_type, score DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_kg_facts_subject_predicate_object ON kg_facts (subject_id, predicate, object_id);

-- Partial indexes for oncology-specific queries
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entities_cancer_types ON entities (entity_type, normalized_id) WHERE entity_type = 'DISEASE' AND normalized_id LIKE 'C%';
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entities_gene_symbols ON entities (entity_type, normalized_id) WHERE entity_type = 'GENE' AND normalized_id IS NOT NULL;

-- Workspace memory indexes
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_workspace_memory_scope ON workspace_memory (scope);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_workspace_memory_created_at ON workspace_memory (created_at DESC);

-- Performance monitoring tables
CREATE TABLE IF NOT EXISTS performance_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_name TEXT NOT NULL,
    metric_value DOUBLE PRECISION NOT NULL,
    labels JSONB DEFAULT '{}',
    collected_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS health_checks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    component_name TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('healthy', 'unhealthy', 'unknown')),
    response_time_ms INTEGER,
    message TEXT,
    checked_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for monitoring tables
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_performance_metrics_name_time ON performance_metrics (metric_name, collected_at DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_performance_metrics_labels ON performance_metrics USING GIN (labels);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_health_checks_component_time ON health_checks (component_name, checked_at DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_health_checks_status ON health_checks (status);

-- Cache tables for frequently accessed data
CREATE TABLE IF NOT EXISTS embedding_cache (
    cache_key TEXT PRIMARY KEY,
    embedding VECTOR(768),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    accessed_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS query_result_cache (
    cache_key TEXT PRIMARY KEY,
    result_data JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    accessed_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS llm_response_cache (
    cache_key TEXT PRIMARY KEY,
    response_text TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    accessed_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for cache tables
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_embedding_cache_accessed ON embedding_cache (accessed_at DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_query_result_cache_accessed ON query_result_cache (accessed_at DESC);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_llm_response_cache_accessed ON llm_response_cache (accessed_at DESC);

-- Function to clean old cache entries (run periodically)
CREATE OR REPLACE FUNCTION clean_old_cache_entries(days_old INTEGER DEFAULT 30)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    -- Clean embedding cache
    DELETE FROM embedding_cache WHERE accessed_at < NOW() - INTERVAL '1 day' * days_old;
    GET DIAGNOSTICS deleted_count = ROW_COUNT;

    -- Clean query result cache
    DELETE FROM query_result_cache WHERE accessed_at < NOW() - INTERVAL '1 day' * days_old;

    -- Clean LLM response cache
    DELETE FROM llm_response_cache WHERE accessed_at < NOW() - INTERVAL '1 day' * days_old;

    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Function to update cache access time
CREATE OR REPLACE FUNCTION update_cache_access(cache_table TEXT, cache_key_param TEXT)
RETURNS VOID AS $$
BEGIN
    IF cache_table = 'embedding_cache' THEN
        UPDATE embedding_cache SET accessed_at = NOW() WHERE cache_key = cache_key_param;
    ELSIF cache_table = 'query_result_cache' THEN
        UPDATE query_result_cache SET accessed_at = NOW() WHERE cache_key = cache_key_param;
    ELSIF cache_table = 'llm_response_cache' THEN
        UPDATE llm_response_cache SET accessed_at = NOW() WHERE cache_key = cache_key_param;
    END IF;
END;
$$ LANGUAGE plpgsql;