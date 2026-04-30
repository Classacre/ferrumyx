-- migrations/004_user_setup_and_permissions.sql
-- Set up database users and permissions for Ferrumyx

-- Create application user
DO $$
BEGIN
   IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'ferrumyx') THEN
      CREATE USER ferrumyx;
   END IF;
END
$$;

-- Create read-only user for monitoring and analytics
DO $$
BEGIN
   IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'ferrumyx_readonly') THEN
      CREATE USER ferrumyx_readonly;
   END IF;
END
$$;

-- Create backup user with minimal privileges
DO $$
BEGIN
   IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'ferrumyx_backup') THEN
      CREATE USER ferrumyx_backup;
   END IF;
END
$$;

-- Grant permissions to application user
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO ferrumyx;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO ferrumyx;

-- Grant read-only permissions
GRANT SELECT ON ALL TABLES IN SCHEMA public TO ferrumyx_readonly;

-- Grant backup permissions (SELECT + USAGE on schema)
GRANT USAGE ON SCHEMA public TO ferrumyx_backup;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO ferrumyx_backup;

-- Ensure future tables get proper permissions
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO ferrumyx;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO ferrumyx_readonly;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO ferrumyx_backup;