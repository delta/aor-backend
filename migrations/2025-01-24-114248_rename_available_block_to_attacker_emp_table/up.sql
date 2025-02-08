-- Your SQL goes here


DROP TABLE public.available_blocks;

DROP TYPE item_category;


CREATE TABLE IF NOT EXISTS public.available_attackers  (
    id serial NOT NULL,
    user_id INTEGER NOT NULL,
    attacker_type_id INTEGER NOT NULL,
    CONSTRAINT attacker_id_fk FOREIGN KEY (attacker_type_id) REFERENCES public.attacker_type(id),
    CONSTRAINT user_id_fk FOREIGN KEY (user_id) REFERENCES public.user(id),
    CONSTRAINT available_attackers_pk PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.available_emps (
    id serial NOT NULL,
    user_id INTEGER NOT NULL,
    emp_type_id INTEGER NOT NULL,
    CONSTRAINT emp_id_fk FOREIGN KEY (emp_type_id) REFERENCES public.emp_type(id),
    CONSTRAINT user_id_fk FOREIGN KEY (user_id) REFERENCES public.user(id),
    CONSTRAINT available_emp_pk PRIMARY KEY (id)
);
