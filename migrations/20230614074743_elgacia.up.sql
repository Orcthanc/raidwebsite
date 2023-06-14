-- Add up migration script here
INSERT INTO raids (name, difficulty, required_item_level, three_weekly) VALUES
    ("Kayangel 1/2", "Normal", 1540, 1),
    ("Kayangel 3/4", "Normal", 1540, 0),
    ("Kayangel 1/2", "Hard", 1580, 1),
    ("Kayangel 3/4", "Hard", 1580, 0);