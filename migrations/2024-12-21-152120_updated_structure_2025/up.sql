-- New prop table
CREATE TABLE prop (
    id SERIAL PRIMARY KEY NOT NULL,
    "range" INTEGER NOT NULL DEFAULT 0,
    frequency INTEGER NOT NULL DEFAULT 0
);

-- Drop defender_type, mine_type, building_type columns and add category_id to block_type
ALTER TABLE block_type
    DROP COLUMN defender_type,
    DROP COLUMN mine_type,
    DROP COLUMN building_type,
    ADD COLUMN category_id INTEGER NOT NULL;

-- Deleting range and freqency columns from defender_type, mine_type, and attacker_type
ALTER TABLE defender_type
    DROP COLUMN radius;

ALTER TABLE mine_type
    DROP COLUMN radius;


-- Adding prop_id column to defender_type, mine_type, building_type, and attacker_type
ALTER TABLE building_type
    ADD COLUMN prop_id INTEGER NOT NULL;

ALTER TABLE defender_type
    ADD COLUMN prop_id INTEGER NOT NULL;

ALTER TABLE mine_type
    ADD COLUMN prop_id INTEGER NOT NULL;

ALTER TABLE attacker_type
    ADD COLUMN prop_id INTEGER NOT NULL;

--Add foreign key constraints for prop_id
ALTER TABLE building_type
    ADD CONSTRAINT fk_building_prop FOREIGN KEY (prop_id) REFERENCES prop(id);

ALTER TABLE defender_type
    ADD CONSTRAINT fk_defender_prop FOREIGN KEY (prop_id) REFERENCES prop(id);

ALTER TABLE mine_type
    ADD CONSTRAINT fk_mine_prop FOREIGN KEY (prop_id) REFERENCES prop(id);

ALTER TABLE attacker_type
    ADD CONSTRAINT fk_attacker_prop FOREIGN KEY (prop_id) REFERENCES prop(id);
