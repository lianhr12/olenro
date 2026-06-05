-- Olenro Database Schema
-- This file is included at compile time for database initialization

-- Providers table
CREATE TABLE IF NOT EXISTS providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    settings_config TEXT NOT NULL,
    website_url TEXT,
    category TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    sort_index INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    is_partner INTEGER NOT NULL DEFAULT 0,
    meta TEXT NOT NULL DEFAULT '{}',
    icon TEXT,
    icon_color TEXT,
    in_failover_queue INTEGER NOT NULL DEFAULT 0
);

-- Universal providers table (cross-app)
CREATE TABLE IF NOT EXISTS universal_providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    apps TEXT NOT NULL,
    models TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- MCP servers table
CREATE TABLE IF NOT EXISTS mcp_servers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    command TEXT NOT NULL,
    args TEXT NOT NULL DEFAULT '[]',
    env TEXT NOT NULL DEFAULT '{}',
    url TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- App MCP mapping
CREATE TABLE IF NOT EXISTS mcp_app_mapping (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    app_type TEXT NOT NULL,
    server_id TEXT NOT NULL,
    UNIQUE(app_type, server_id)
);

-- Prompts table
CREATE TABLE IF NOT EXISTS prompts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    description TEXT,
    enabled INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Skills table
CREATE TABLE IF NOT EXISTS skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    path TEXT NOT NULL,
    source_type TEXT NOT NULL,
    source_data TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Skill app mapping
CREATE TABLE IF NOT EXISTS skill_app_mapping (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    app_type TEXT NOT NULL,
    skill_id TEXT NOT NULL,
    UNIQUE(app_type, skill_id)
);

-- Settings table
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Proxy config table
CREATE TABLE IF NOT EXISTS proxy_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    enabled INTEGER NOT NULL DEFAULT 0,
    port INTEGER NOT NULL DEFAULT 15721,
    address TEXT NOT NULL DEFAULT '127.0.0.1',
    updated_at INTEGER NOT NULL
);

-- Failover queue table
CREATE TABLE IF NOT EXISTS failover_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

-- Usage rollup table
CREATE TABLE IF NOT EXISTS usage_rollup (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id TEXT NOT NULL,
    date TEXT NOT NULL,
    requests INTEGER NOT NULL DEFAULT 0,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cost REAL NOT NULL DEFAULT 0.0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(provider_id, date)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_providers_category ON providers(category);
CREATE INDEX IF NOT EXISTS idx_providers_sort ON providers(sort_index);
CREATE INDEX IF NOT EXISTS idx_prompts_enabled ON prompts(enabled);
CREATE INDEX IF NOT EXISTS idx_usage_rollup_date ON usage_rollup(date);
CREATE INDEX IF NOT EXISTS idx_usage_rollup_provider ON usage_rollup(provider_id);
