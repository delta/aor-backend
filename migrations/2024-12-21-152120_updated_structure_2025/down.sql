-- Remove foreign key constraints for prop_id
ALTER TABLE building_type
    DROP CONSTRAINT IF EXISTS fk_building_prop;

ALTER TABLE defender_type
    DROP CONSTRAINT IF EXISTS fk_defender_prop;

ALTER TABLE mine_type
    DROP CONSTRAINT IF EXISTS fk_mine_prop;

ALTER TABLE attacker_type
    DROP CONSTRAINT IF EXISTS fk_attacker_prop;

-- Remove prop_id column from defender_type, mine_type, building_type, and attacker_type
ALTER TABLE building_type
    DROP COLUMN IF EXISTS prop_id;

ALTER TABLE defender_type
    DROP COLUMN IF EXISTS prop_id;

ALTER TABLE mine_type
    DROP COLUMN IF EXISTS prop_id;

ALTER TABLE attacker_type
    DROP COLUMN IF EXISTS prop_id;

-- Restore "name" and radius columns in defender_type, mine_type, and attacker_type
ALTER TABLE defender_type
    ADD COLUMN "name" TEXT NOT NULL,
    ADD COLUMN radius INTEGER NOT NULL;

ALTER TABLE mine_type
    ADD COLUMN "name" TEXT NOT NULL,
    ADD COLUMN radius INTEGER NOT NULL;

ALTER TABLE attacker_type
    ADD COLUMN "name" TEXT NOT NULL,
    ADD COLUMN radius INTEGER NOT NULL;

-- Drop the prop table
DROP TABLE IF EXISTS prop;
