CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at INTEGER NOT NULL
);
CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_expiry ON sessions(expires_at);
-- Fail safely on duplicate personal books; never discard existing data.
CREATE UNIQUE INDEX idx_personal_book_owner
    ON address_books(owner_id) WHERE is_personal = TRUE;
CREATE INDEX idx_shares_user ON ab_shares(user_id, ab_guid);
CREATE INDEX idx_shares_group ON ab_shares(group_id, ab_guid);
CREATE INDEX idx_user_groups_group ON user_groups(group_id);
CREATE INDEX idx_audit_created ON audit_log(created_at DESC, id DESC);
