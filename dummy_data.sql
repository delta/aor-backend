SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

DELETE FROM public.artifact;
DELETE FROM public.map_spaces;
DELETE FROM public.available_attackers;
DELETE FROM public.available_emps;
DELETE FROM public.level_constraints;
DELETE FROM public.block_type;
DELETE FROM public.attacker_type;
DELETE FROM public.mine_type;
DELETE FROM public.emp_type;
DELETE FROM public.defender_type;
DELETE FROM public.building_type;
DELETE FROM public.simulation_log;
DELETE FROM public.game;
DELETE FROM public.map_layout;
DELETE FROM public.user;
DELETE FROM public.levels_fixture;
DELETE FROM public.prop;

COPY public.levels_fixture FROM stdin;
1	2025_01_01 00:00:00	2026_01_01 00:00:00	40	1	3
\.

COPY public.user FROM stdin;
1	Bot	donwick32@gmail.com	bot	true	0	0	1000	0	500
\.

COPY public.map_layout FROM stdin;
1	1	1	t
\.

COPY public.prop FROM stdin;
0	0	0
1	8	0
2	10	0
3	6	0
4	7	0
5	8	0
6	9	0
7	9	0
8	10	0
9	8	0
10	5	0
11	6	0
12	7	0
13	10	5000
14	15	4000
15	19	3000
16	5	1
17	10	2
18	15	3
19	5	0
20	10	0
21	15	0
\.


COPY public.building_type FROM stdin;
0	Road	1	1	0	0	0	0	0
1	Bank	3	3	2147483647	1	10	120	0
2	Building_2	4	4	120	1	10	140	0
3	Building_3	5	5	140	1	10	160	0
4	Building_4	3	3	90	1	10	110	0
5	Building_5	4	4	110	1	10	130	0
6	Building_6	5	5	130	1	10	150	0
7	Building_7	3	3	80	1	10	100	0
8	Building_8	4	4	100	1	10	120	0
9	Building_9	5	5	120	1	10	140	0
10	Building_10	3	3	70	1	10	90	0
11	Building_11	4	4	90	1	10	110	0
12	Building_12	5	5	110	1	10	130	0
13	Building_13	3	3	60	1	10	80	0
14	Building_14	4	4	80	1	10	100	0
15	Building_15	5	5	100	1	10	120	0
16	Bank	3	3	2147483647	2	50	140	0
17	Building_2	4	4	130	2	75	160	0
18	Building_3	5	5	150	2	100	180	0
19	Building_4	3	3	100	2	25	120	0
20	Building_5	4	4	120	2	50	140	0
21	Building_6	5	5	140	2	75	160	0
22	Building_7	3	3	90	2	20	110	0
23	Building_8	4	4	110	2	45	130	0
24	Building_9	5	5	130	2	70	150	0
25	Building_10	3	3	80	2	15	100	0
26	Building_11	4	4	100	2	40	120	0
27	Building_12	5	5	120	2	65	140	0
28	Building_13	3	3	70	2	10	90	0
29	Building_14	4	4	90	2	35	110	0
30	Building_15	5	5	110	2	60	130	0
31	Bank	3	3	2147483647	3	-1	160	0
32	Building_2	4	4	140	3	-1	180	0
33	Building_3	5	5	160	3	-1	200	0
34	Building_4	3	3	110	3	-1	140	0
35	Building_5	4	4	130	3	-1	160	0
36	Building_6	5	5	150	3	-1	180	0
37	Building_7	3	3	100	3	-1	120	0
38	Building_8	4	4	120	3	-1	140	0
39	Building_9	5	5	140	3	-1	160	0
40	Building_10	3	3	90	3	-1	110	0
41	Building_11	4	4	110	3	-1	130	0
42	Building_12	5	5	130	3	-1	150	0
43	Building_13	3	3	80	3	-1	100	0
44	Building_14	4	4	100	3	-1	120	0
45	Building_15	5	5	120	3	-1	140	0
46	Defender_Hut	3	3	120	1	10	120	13
47	Defender_Hut	3	3	120	2	75	140	14
48	Defender_Hut	3	3	120	3	-1	160	15
49	Sentry	3	3	120	1	10	120	16
50	Sentry	3	3	120	2	75	140	17
51	Sentry	3	3	120	3	-1	160	18
\.

COPY public.defender_type FROM stdin;
1	1	50	1	10	Defender_1	1	100
2	1	40	1	10	Defender_2	2	100
3	1	30	1	10	Defender_3	3	100
4	1	60	2	150	Defender_1	4	100
5	1	50	2	250	Defender_2	5	100
6	1	40	2	350	Defender_3	6	100
7	1	70	3	-1	Defender_1	7	100
8	1	60	3	-1	Defender_2	8	100
9	1	50	3	-1	Defender_3	9	100
10	1	20	1	-1	Hut_Defender	19	100
11	1	30	2	-1	Hut_Defender	20	100
12	1	50	3	-1	Hut_Defender	21	100
\.

COPY public.emp_type FROM stdin;
1	Bomb_1	3	20	10	Bomb_1	1
2	Bomb_2	5	30	10	Bomb_2	1
3	Bomb_3	4	25	10	Bomb_3	1
4	Bomb_1	4	30	120	Bomb_1	2
5	Bomb_2	6	40	180	Bomb_2	2
6	Bomb_3	5	35	150	Bomb_3	2
7	Bomb_1	5	40	-1	Bomb_1	3
8	Bomb_2	7	50	-1	Bomb_2	3
9	Bomb_3	6	45	-1	Bomb_3	3
\.

COPY public.mine_type FROM stdin;
1	50	1	10	Mine_1	10
2	70	2	120	Mine_1	11
3	90	3	-1	Mine_1	12
\.

COPY public.attacker_type FROM stdin;
1	100	1	10	1	10	Attacker_1	0
2	120	1	12	1	10	Attacker_2	0
3	80	1	8	1	10	Attacker_3	0
4	150	1	15	2	80	Attacker_1	0
5	180	1	18	2	100	Attacker_2	0
6	120	1	12	2	60	Attacker_3	0
7	200	1	20	3	-1	Attacker_1	0
8	240	1	24	3	-1	Attacker_2	0
9	160	1	16	3	-1	Attacker_3	0
\.

COPY public.block_type FROM stdin;
0	\N	\N	building	0
1	\N	\N	building	1
2	\N	\N	building	2
3	\N	\N	building	3
4	\N	\N	building	4
5	\N	\N	building	5
6	\N	\N	building	6
7	\N	\N	building	7
8	\N	\N	building	8
9	\N	\N	building	9
10	\N	\N	building	10
11	\N	\N	building	11
12	\N	\N	building	12
13	\N	\N	building	13
14	\N	\N	building	14
15	\N	\N	building	15
16	\N	\N	building	16
17	\N	\N	building	17
18	\N	\N	building	18
19	\N	\N	building	19
20	\N	\N	building	20
21	\N	\N	building	21
22	\N	\N	building	22
23	\N	\N	building	23
24	\N	\N	building	24
25	\N	\N	building	25
26	\N	\N	building	26
27	\N	\N	building	27
28	\N	\N	building	28
29	\N	\N	building	29
30	\N	\N	building	30
31	\N	\N	building	31
32	\N	\N	building	32
33	\N	\N	building	33
34	\N	\N	building	34
35	\N	\N	building	35
36	\N	\N	building	36
37	\N	\N	building	37
38	\N	\N	building	38
39	\N	\N	building	39
40	\N	\N	building	40
41	\N	\N	building	41
42	\N	\N	building	42
43	\N	\N	building	43
44	\N	\N	building	44
45	\N	\N	building	45
46	1	\N	defender	0
47	2	\N	defender	0
48	3	\N	defender	0
49	4	\N	defender	0
50	5	\N	defender	0
51	6	\N	defender	0
52	7	\N	defender	0
53	8	\N	defender	0
54	9	\N	defender	0
55	\N	1	mine	0
56	\N	2	mine	0
57	\N	3	mine	0
58	\N	\N	building	46
59	\N	\N	building	47
60	\N	\N	building	48
61	\N	\N	building	49
62	\N	\N	building	50
63	\N	\N	building	51
64	10	\N	defender	0
65	11	\N	defender	0
66	12	\N	defender	0
\.


COPY public.available_attackers FROM stdin;
1	1	1
2	1	2
3	1	3
\.

COPY public.available_emps FROM stdin;
1	1	1
2	1	2
3	1	3
\.
COPY public.map_spaces FROM stdin;
1	1	0	0	0
2	1	0	1	0
3	1	0	2	0
4	1	0	3	0
5	1	0	4	0
6	1	0	5	0
7	1	0	6	0
8	1	0	7	0
9	1	0	8	0
10	1	0	9	0
11	1	0	10	0
12	1	0	11	0
13	1	0	12	0
14	1	0	13	0
15	1	0	14	0
16	1	0	15	0
17	1	0	16	0
18	1	0	17	0
19	1	0	18	0
20	1	0	19	0
21	1	0	20	0
22	1	0	21	0
23	1	0	22	0
24	1	0	23	0
25	1	0	24	0
26	1	0	25	0
27	1	0	26	0
28	1	0	27	0
29	1	0	28	0
30	1	0	29	0
31	1	0	30	0
32	1	0	31	0
33	1	0	32	0
34	1	0	33	0
35	1	0	34	0
36	1	0	35	0
37	1	0	36	0
38	1	0	37	0
39	1	0	38	0
40	1	0	39	0
41	1	1	39	0
42	1	2	39	0
43	1	3	39	0
44	1	4	39	0
45	1	5	39	0
46	1	6	39	0
47	1	7	39	0
48	1	8	39	0
49	1	9	39	0
50	1	10	39	0
51	1	11	39	0
52	1	12	39	0
53	1	13	39	0
54	1	14	39	0
55	1	15	39	0
56	1	16	39	0
57	1	17	39	0
58	1	18	39	0
59	1	19	39	0
60	1	20	39	0
61	1	21	39	0
62	1	22	39	0
63	1	23	39	0
64	1	24	39	0
65	1	25	39	0
66	1	26	39	0
67	1	27	39	0
68	1	28	39	55
69	1	29	39	0
70	1	30	39	55
71	1	31	39	0
72	1	32	39	55
73	1	33	39	0
74	1	34	39	55
75	1	35	39	0
76	1	36	39	55
77	1	37	39	0
78	1	38	39	55
79	1	39	39	46
80	1	39	38	0
81	1	39	37	47
82	1	39	36	0
83	1	39	35	48
84	1	39	34	0
85	1	39	33	46
86	1	39	32	0
87	1	39	31	47
88	1	39	30	0
89	1	39	29	48
90	1	39	28	0
91	1	39	27	0
92	1	39	26	0
93	1	39	25	0
94	1	39	24	0
95	1	39	23	0
96	1	39	22	0
97	1	39	21	0
98	1	39	20	0
99	1	39	19	0
100	1	39	18	0
101	1	39	17	0
102	1	39	16	0
103	1	39	15	0
104	1	39	14	0
105	1	39	13	0
106	1	39	12	0
107	1	39	11	0
108	1	39	10	0
109	1	39	9	0
110	1	39	8	0
111	1	39	7	0
112	1	39	6	0
113	1	39	5	0
114	1	39	4	0
115	1	39	3	0
116	1	39	2	0
117	1	39	1	0
118	1	39	0	0
119	1	38	0	0
120	1	37	0	0
121	1	36	0	0
122	1	35	0	0
123	1	34	0	0
124	1	33	0	0
125	1	32	0	0
126	1	31	0	0
127	1	30	0	0
128	1	29	0	0
129	1	28	0	0
130	1	27	0	0
131	1	26	0	0
132	1	25	0	0
133	1	24	0	0
134	1	23	0	0
135	1	22	0	0
136	1	21	0	0
137	1	20	0	0
138	1	19	0	0
139	1	18	0	0
140	1	17	0	0
141	1	16	0	0
142	1	15	0	0
143	1	14	0	0
144	1	13	0	0
145	1	12	0	0
146	1	11	0	0
147	1	10	0	0
148	1	9	0	0
149	1	8	0	0
150	1	7	0	0
151	1	6	0	0
152	1	5	0	0
153	1	4	0	0
154	1	3	0	0
155	1	2	0	0
156	1	1	0	0
157	1	1	1	1
158	1	6	1	2
159	1	11	1	3
160	1	16	1	4
161	1	21	1	5
162	1	26	1	6
163	1	31	1	7
164	1	34	1	8
165	1	1	5	9
166	1	1	10	10
167	1	1	15	11
168	1	1	20	12
169	1	1	25	13
170	1	1	30	14
171	1	1	34	15
172	1	36	20	58
173	1	20	36	61
174	1	10	36	61
175	1	36	10	58
\.

COPY public.artifact FROM stdin;
157	500
\.

COPY public.level_constraints FROM stdin;
1	1	1
1	1	2
1	1	3
1	1	4
1	1	5
1	1	6
1	1	7
1	1	8
1	1	9
1	1	10
1	1	11
1	1	12
1	1	13
1	1	14
1	1	15
1	1	16
1	1	17
1	1	18
1	1	19
1	1	20
1	1	21
1	1	22
1	1	23
1	1	24
1	1	25
1	1	26
1	1	27
1	1	28
1	1	29
1	1	30
1	1	31
1	1	32
1	1	33
1	1	34
1	1	35
1	1	36
1	1	37
1	1	38
1	1	39
1	1	40
1	1	41
1	1	42
1	1	43
1	1	44
1	1	45
1	2	46
1	2	47
1	2	48
1	2	49
1	2	50
1	2	51
1	2	52
1	2	53
1	2	54
1	6	55
1	6	56
1	6	57
\.

SELECT pg_catalog.setval('public.user_id_seq', 2, false);
SELECT pg_catalog.setval('public.map_layout_id_seq', 2, false);
SELECT pg_catalog.setval('public.game_id_seq', 1, false);
SELECT pg_catalog.setval('public.block_type_id_seq', 64, false);
SELECT pg_catalog.setval('public.map_spaces_id_seq', 178, false);
SELECT pg_catalog.setval('public.available_attackers_id_seq', 4, false);
SELECT pg_catalog.setval('public.available_emps_id_seq', 4, false);
