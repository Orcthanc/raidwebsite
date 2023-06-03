-- Add up migration script here
INSERT INTO raid_prerequisites (raid, requires)
    (SELECT r.id, rr.id
    FROM raids r
    JOIN raids rr
    WHERE r.name = "Brelshaza G3/4" AND rr.name = "Brelshaza G1/2");
    
INSERT INTO raid_prerequisites (raid, requires)
    (SELECT r.id, rr.id
    FROM raids r
    JOIN raids rr
    WHERE r.name = "Brelshaza G5/6" AND rr.name = "Brelshaza G3/4");