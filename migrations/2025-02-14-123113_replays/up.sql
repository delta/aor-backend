-- Your SQL goes here
DROP TABLE IF EXISTS public.replays;
CREATE TABLE public.replays (
    game_id INTEGER NOT NULL,
    attacker_id INTEGER NOT NULL,
    defender_id INTEGER NOT NULL,
    base_data BYTEA NOT NULL,
    game_data BYTEA NOT NULL,
    CONSTRAINT game_id_fk FOREIGN KEY (game_id) REFERENCES public.game(id),
    CONSTRAINT attacker_id_fk FOREIGN KEY (attacker_id) REFERENCES public.user(id),
    CONSTRAINT defender_id_fk FOREIGN KEY (defender_id) REFERENCES public.user(id),
    PRIMARY KEY (game_id)
);