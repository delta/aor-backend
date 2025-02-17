-- This file should undo anything in `up.sql`


CREATE TYPE item_category AS ENUM ('attacker', 'emp', 'block');

CREATE TABLE public.available_blocks(
    block_type_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    attacker_type_id INTEGER,
    emp_type_id INTEGER,
    category item_category NOT NULL,
    id serial NOT NULL

    CONSTRAINT user_id_fk FOREIGN KEY (user_id) REFERENCES public.user(id),
    CONSTRAINT attacker_id_fk FOREIGN KEY (attacker_type_id) REFERENCES public.attacker_type(id),
    CONSTRAINT block_type_id_fk FOREIGN KEY (block_type_id) REFERENCES public.block_type(id),
    CONSTRAINT available_blocks_id_primary PRIMARY KEY(id),
) WITH (
  OIDS=FALSE
);

DROP TABLE public.available_attackers;
DROP TABLE public.available_emps;