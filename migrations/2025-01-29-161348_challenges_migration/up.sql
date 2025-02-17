-- Your SQL goes here
CREATE TABLE challenges
(
    id SERIAL PRIMARY KEY,
    "name" VARCHAR(255) NOT NULL,
    "start" TIMESTAMP NOT NULL,
    "end" TIMESTAMP NOT NULL
);

CREATE TABLE challenge_maps
(
    id SERIAL PRIMARY KEY,
    challenge_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    map_id INTEGER NOT NULL,
    CONSTRAINT fk_chall_map_challenge FOREIGN KEY (challenge_id) REFERENCES challenges(id),
    CONSTRAINT fk_chall_map_user FOREIGN KEY (user_id) REFERENCES "user"(id)
);

CREATE TABLE challenges_responses
(
    id SERIAL PRIMARY KEY,
    attacker_id INTEGER NOT NULL,
    challenge_id INTEGER NOT NULL,
    map_id INTEGER NOT NULL,
    score INTEGER NOT NULL,
    attempts INTEGER NOT NULL,
    CONSTRAINT chall_response_attacker FOREIGN KEY (attacker_id) REFERENCES "user"(id),
    CONSTRAINT chall_response_challenge FOREIGN KEY (challenge_id) REFERENCES challenges(id),
    CONSTRAINT unique_attacker_challenge UNIQUE (attacker_id, challenge_id, map_id)
);

ALTER TABLE "user" ADD COLUMN is_mod BOOLEAN NOT NULL;
