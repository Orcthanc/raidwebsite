-- Add up migration script here
INSERT INTO raids (name, difficulty, required_item_level, three_weekly) VALUES
    ("Kayangel 1/2", "Normal", 1540, 1),
    ("Kayangel 3/4", "Normal", 1540, 0),
    ("Kayangel 1/2", "Hard", 1580, 1),
    ("Kayangel 3/4", "Hard", 1580, 0);

INSERT INTO raid_prerequisites (raid, requires)
    (SELECT r.id, rr.id
    FROM raids r
    JOIN raids rr
    WHERE r.name = "Kayangel 3/4" AND rr.name = "Kayangel 1/2");