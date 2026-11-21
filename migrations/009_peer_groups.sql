CREATE TABLE peer_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ab_guid TEXT NOT NULL REFERENCES address_books(guid) ON DELETE CASCADE,
    name TEXT NOT NULL,
    note TEXT NOT NULL DEFAULT '',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(ab_guid, name)
);

ALTER TABLE peers ADD COLUMN group_id INTEGER REFERENCES peer_groups(id) ON DELETE SET NULL;
CREATE INDEX idx_peers_book_group ON peers(ab_guid, group_id, rustdesk_id);

CREATE TRIGGER peer_group_same_book_insert BEFORE INSERT ON peers
WHEN NEW.group_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM peer_groups WHERE id = NEW.group_id AND ab_guid = NEW.ab_guid
)
BEGIN
    SELECT RAISE(ABORT, 'Peer group must belong to the same address book');
END;

CREATE TRIGGER peer_group_same_book_update BEFORE UPDATE OF group_id, ab_guid ON peers
WHEN NEW.group_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM peer_groups WHERE id = NEW.group_id AND ab_guid = NEW.ab_guid
)
BEGIN
    SELECT RAISE(ABORT, 'Peer group must belong to the same address book');
END;
