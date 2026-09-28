-- Agent Insure Schema - Initial Migration
-- SQLite-compatible types

-- Agents table
CREATE TABLE IF NOT EXISTS agents (
    agent_id TEXT PRIMARY KEY,
    organization_id TEXT,
    use_case_class TEXT NOT NULL,
    industry_class TEXT NOT NULL,
    geographic_region TEXT,
    deployment_date TEXT,
    schema_name TEXT NOT NULL DEFAULT 'Agent',
    schema_version TEXT NOT NULL,
    generated_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_agents_org ON agents(organization_id);

-- Exposure periods table
CREATE TABLE IF NOT EXISTS exposure_periods (
    exposure_id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    period_start TEXT NOT NULL,
    period_end TEXT NOT NULL,
    agent_active_hours REAL NOT NULL,
    actions_executed_total INTEGER NOT NULL DEFAULT 0,
    actions_executed_high_risk INTEGER NOT NULL DEFAULT 0,
    authority_surface_score INTEGER,
    unique_resources_accessed INTEGER,
    external_api_calls INTEGER,
    schema_name TEXT NOT NULL DEFAULT 'ExposurePeriod',
    schema_version TEXT NOT NULL,
    generated_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (agent_id) REFERENCES agents(agent_id)
);

CREATE INDEX IF NOT EXISTS idx_exposures_agent ON exposure_periods(agent_id);
CREATE INDEX IF NOT EXISTS idx_exposures_period ON exposure_periods(period_start, period_end);

-- Control snapshots table
CREATE TABLE IF NOT EXISTS control_snapshots (
    control_snapshot_id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    effective_start TEXT NOT NULL,
    effective_end TEXT NOT NULL,
    autonomy_level TEXT NOT NULL,
    hitl_required_for_high_risk INTEGER NOT NULL DEFAULT 1,
    least_privilege_enforced INTEGER NOT NULL DEFAULT 1,
    sandboxing_enforced INTEGER NOT NULL DEFAULT 1,
    secret_rotation_days INTEGER,
    audit_log_coverage_pct INTEGER,
    red_team_coverage_pct INTEGER,
    deployment_events_30d INTEGER,
    model_version_updates_30d INTEGER,
    schema_name TEXT NOT NULL DEFAULT 'ControlSnapshot',
    schema_version TEXT NOT NULL,
    generated_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (agent_id) REFERENCES agents(agent_id)
);

CREATE INDEX IF NOT EXISTS idx_controls_agent ON control_snapshots(agent_id);
CREATE INDEX IF NOT EXISTS idx_controls_effective ON control_snapshots(effective_start);

-- Incidents table
CREATE TABLE IF NOT EXISTS incidents (
    incident_id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    incident_class TEXT NOT NULL,
    severity_level TEXT NOT NULL,
    line_of_business TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    detected_at TEXT,
    resolved_at TEXT,
    root_cause_id TEXT,
    schema_name TEXT NOT NULL DEFAULT 'Incident',
    schema_version TEXT NOT NULL,
    generated_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (agent_id) REFERENCES agents(agent_id)
);

CREATE INDEX IF NOT EXISTS idx_incidents_agent ON incidents(agent_id);
CREATE INDEX IF NOT EXISTS idx_incidents_occurred ON incidents(occurred_at);

-- Claim developments table
CREATE TABLE IF NOT EXISTS claim_developments (
    claim_id TEXT NOT NULL,
    incident_id TEXT NOT NULL,
    valuation_date TEXT NOT NULL,
    paid_amount_cumulative REAL NOT NULL,
    reserve_amount REAL NOT NULL,
    expected_ultimate_amount REAL NOT NULL,
    status TEXT NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    schema_name TEXT NOT NULL DEFAULT 'ClaimDevelopment',
    schema_version TEXT NOT NULL,
    generated_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (claim_id, valuation_date),
    FOREIGN KEY (incident_id) REFERENCES incidents(incident_id)
);

CREATE INDEX IF NOT EXISTS idx_claims_incident ON claim_developments(incident_id);
