-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS challenges;
DROP TABLE IF EXISTS challenge_maps;
DROP TABLE IF EXISTS challenges_responses;
ALTER TABLE "user" DROP COLUMN is_mod;