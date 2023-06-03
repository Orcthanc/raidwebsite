-- Add up migration script here
INSERT INTO raids (name, difficulty, required_item_level, three_weekly) VALUES
    ("Argos", "Normal", 1370, 0),
    ("Valtan", "Normal", 1415, 1),
    ("Valtan", "Hard", 1445, 1),
    ("Vykas", "Normal", 1430, 1),
    ("Vykas", "Hard", 1460, 1),
    ("Kakul-Seydon", "Normal", 1475, 1),
    ("Brelshaza G1/2", "Normal", 1490, 1),
    ("Brelshaza G1/2", "Hard", 1540, 1),
    ("Brelshaza G3/4", "Normal", 1500, 0),
    ("Brelshaza G3/4", "Hard", 1550, 0),
    ("Brelshaza G5/6", "Normal", 1520, 0),
    ("Brelshaza G5/6", "Hard", 1560, 0);