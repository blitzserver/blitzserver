CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


--
-- Name: EXTENSION pgcrypto; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pgcrypto IS 'cryptographic functions';

CREATE FUNCTION public.update_next_turn() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
	UPDATE games g 
	SET next_turn = NEW.created_at + interval '1 minute' * timer
	WHERE id=NEW.game_id AND timer IS NOT NULL;
	UPDATE games g 
	SET next_turn = NULL
	WHERE id=NEW.game_id AND timer IS NULL;
	RETURN NEW;
END
$$;




CREATE TABLE public.admin_logs (
    id integer NOT NULL,
    game_id integer NOT NULL,
    datetime timestamp without time zone NOT NULL,
    action character varying NOT NULL
);




--
-- Name: admin_logs_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.admin_logs_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: admin_logs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.admin_logs_id_seq OWNED BY public.admin_logs.id;


--
-- Name: binaries; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.binaries (
    binary_id integer NOT NULL,
    executable_path text NOT NULL,
    version text NOT NULL,
    version_number integer GENERATED ALWAYS AS ((regexp_replace(version, '[^0-9]+'::text, ''::text, 'g'::text))::numeric) STORED NOT NULL,
    foldered_maps boolean GENERATED ALWAYS AS (((regexp_replace(version, '[^0-9]+'::text, ''::text, 'g'::text))::numeric > (561)::numeric)) STORED NOT NULL
);




--
-- Name: connections; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.connections (
    id integer NOT NULL,
    connected timestamp without time zone NOT NULL,
    disconnected timestamp without time zone,
    game_id integer,
    nation_id integer,
    ip text
);




--
-- Name: connections_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.connections_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: connections_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.connections_id_seq OWNED BY public.connections.id;


--
-- Name: disciples; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.disciples (
    id integer NOT NULL,
    game_id integer NOT NULL,
    nation_id integer NOT NULL,
    is_disciple integer NOT NULL,
    team integer
);




--
-- Name: disciples_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.disciples_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: disciples_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.disciples_id_seq OWNED BY public.disciples.id;


--
-- Name: discord_configs; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.discord_configs (
    id integer NOT NULL,
    game_id integer NOT NULL,
    last_turn_notified integer,
    discord_guildid character varying NOT NULL,
    discord_channelid character varying NOT NULL,
    message character varying NOT NULL,
    hours_remaining integer
);




--
-- Name: discord_configs_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.discord_configs_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: discord_configs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.discord_configs_id_seq OWNED BY public.discord_configs.id;


--
-- Name: email_configs; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.email_configs (
    id integer NOT NULL,
    nation_id integer NOT NULL,
    game_id integer NOT NULL,
    hours_before_host integer NOT NULL,
    email_address character varying NOT NULL,
    last_turn_notified integer,
    subject character varying DEFAULT 'Default email notifcation subject, you probably have a new turn'::character varying NOT NULL,
    body character varying DEFAULT 'Default email notification body, you probably have a new turn'::character varying NOT NULL,
    is_reminder boolean DEFAULT false NOT NULL
);




--
-- Name: email_configs_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.email_configs_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: email_configs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.email_configs_id_seq OWNED BY public.email_configs.id;


--
-- Name: files; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.files (
    id integer NOT NULL,
    filename character varying NOT NULL,
    filebinary bytea NOT NULL
);




--
-- Name: files_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.files_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: files_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.files_id_seq OWNED BY public.files.id;


--
-- Name: game_logs; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.game_logs (
    id integer NOT NULL,
    game_id integer NOT NULL,
    datetime timestamp without time zone NOT NULL,
    turn_number integer NOT NULL,
    output character varying NOT NULL,
    error character varying NOT NULL,
    log_command character varying DEFAULT ''::character varying NOT NULL
);




--
-- Name: game_logs_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.game_logs_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: game_logs_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.game_logs_id_seq OWNED BY public.game_logs.id;


--
-- Name: game_mods; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.game_mods (
    id integer NOT NULL,
    game_id integer NOT NULL,
    mod_id integer NOT NULL
);




--
-- Name: game_mods_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.game_mods_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: game_mods_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.game_mods_id_seq OWNED BY public.game_mods.id;


--
-- Name: games; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.games (
    id integer NOT NULL,
    name character varying NOT NULL,
    era integer NOT NULL,
    map_id integer DEFAULT 0 NOT NULL,
    port integer,
    timer integer,
    thrones_t1 integer DEFAULT 5 NOT NULL,
    thrones_t2 integer DEFAULT 0 NOT NULL,
    thrones_t3 integer DEFAULT 0 NOT NULL,
    throne_points_required integer DEFAULT 5 NOT NULL,
    research_diff integer DEFAULT 2 NOT NULL,
    research_rand boolean DEFAULT true NOT NULL,
    hof_size integer DEFAULT 10 NOT NULL,
    global_size integer DEFAULT 5 NOT NULL,
    indepstr integer DEFAULT 4 NOT NULL,
    magicsites integer DEFAULT 40 NOT NULL,
    eventrarity integer DEFAULT 2 NOT NULL,
    richness integer DEFAULT 100 NOT NULL,
    resources integer DEFAULT 100 NOT NULL,
    recruitment integer DEFAULT 100 NOT NULL,
    supplies integer DEFAULT 100 NOT NULL,
    startprov integer DEFAULT 2 NOT NULL,
    renaming boolean DEFAULT false NOT NULL,
    scoregraphs boolean DEFAULT true NOT NULL,
    nationinfo boolean DEFAULT true NOT NULL,
    artrest boolean DEFAULT true NOT NULL,
    teamgame boolean DEFAULT false NOT NULL,
    clustered boolean DEFAULT false NOT NULL,
    storyevents integer DEFAULT 1 NOT NULL,
    newailvl integer DEFAULT 2 NOT NULL,
    next_turn timestamp with time zone,
    password character varying DEFAULT 'password'::character varying NOT NULL,
    archived boolean DEFAULT false NOT NULL,
    masterpass character varying,
    private boolean DEFAULT false NOT NULL,
    last_activity timestamp with time zone DEFAULT now() NOT NULL,
    require_password boolean DEFAULT false NOT NULL,
    select_start integer DEFAULT 0 NOT NULL,
    modified_mapcrc integer,
    cataclysm integer,
    max_stales integer,
    motd character varying,
    conq_all boolean DEFAULT false NOT NULL,
    binary_id integer,
    maps_dom6_id integer,
    formal_diplo integer DEFAULT 0 NOT NULL,
    research_9_slow boolean DEFAULT false NOT NULL,
    mercslots integer DEFAULT 5 NOT NULL,
    cheat_detection boolean DEFAULT true NOT NULL,
    provs_per_player integer DEFAULT 15 NOT NULL,
    randommap_wrapping integer DEFAULT 0 NOT NULL,
    log_level integer DEFAULT 0 NOT NULL
);




--
-- Name: games_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.games_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: games_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.games_id_seq OWNED BY public.games.id;


--
-- Name: jobs; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.jobs (
    job_id integer NOT NULL,
    game_id integer NOT NULL,
    turn_number integer NOT NULL,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    reaped boolean DEFAULT false NOT NULL,
    finished_at timestamp with time zone
);




--
-- Name: jobs_job_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.jobs_job_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: jobs_job_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.jobs_job_id_seq OWNED BY public.jobs.job_id;


--
-- Name: map_plane_images; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.map_plane_images (
    map_id integer NOT NULL,
    map_file_id integer NOT NULL,
    image_file_id integer NOT NULL,
    hash integer NOT NULL
);




--
-- Name: map_planes; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.map_planes (
    map_id integer NOT NULL,
    map_file_id integer NOT NULL,
    hash integer NOT NULL,
    prov_count integer
);




--
-- Name: map_provinces; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.map_provinces (
    id integer NOT NULL,
    map_id integer NOT NULL,
    province_id integer NOT NULL,
    x integer NOT NULL,
    y integer NOT NULL,
    unit_x double precision NOT NULL,
    unit_y double precision NOT NULL
);




--
-- Name: map_provinces_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.map_provinces_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: map_provinces_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.map_provinces_id_seq OWNED BY public.map_provinces.id;


--
-- Name: maps; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.maps (
    id integer NOT NULL,
    name character varying NOT NULL,
    mapfile_id integer NOT NULL,
    tgafile_id integer NOT NULL,
    winterfile_id integer,
    province_count integer DEFAULT 0 NOT NULL,
    uw_count integer DEFAULT 0 NOT NULL,
    archived boolean DEFAULT false NOT NULL,
    map_crc integer,
    tga_crc integer,
    winter_crc integer,
    min_version integer DEFAULT 0 NOT NULL
);




--
-- Name: maps_dom6; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.maps_dom6 (
    map_id integer NOT NULL,
    name text NOT NULL,
    is_randommap boolean DEFAULT false NOT NULL
);




--
-- Name: maps_dom6_map_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE public.maps_dom6 ALTER COLUMN map_id ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.maps_dom6_map_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: maps_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.maps_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: maps_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.maps_id_seq OWNED BY public.maps.id;


--
-- Name: mods; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.mods (
    id integer NOT NULL,
    dm_filename character varying NOT NULL,
    name character varying NOT NULL,
    file_id integer NOT NULL,
    icon_file_id integer,
    crc integer
);




--
-- Name: mods_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.mods_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: mods_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.mods_id_seq OWNED BY public.mods.id;


--
-- Name: nations; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.nations (
    id integer NOT NULL,
    game_id integer NOT NULL,
    nation_id integer NOT NULL,
    name character varying NOT NULL,
    epithet character varying NOT NULL,
    filename character varying DEFAULT 'unknown'::character varying NOT NULL
);




--
-- Name: nations_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.nations_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: nations_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.nations_id_seq OWNED BY public.nations.id;


--
-- Name: player_turns; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.player_turns (
    id integer NOT NULL,
    turn_number integer NOT NULL,
    nation_id integer NOT NULL,
    game_id integer NOT NULL,
    trnfile_id integer,
    twohfile_id integer,
    archived boolean DEFAULT false NOT NULL,
    status integer DEFAULT 0 NOT NULL,
    twoh_crc integer
);




--
-- Name: player_turns_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.player_turns_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: player_turns_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.player_turns_id_seq OWNED BY public.player_turns.id;


--
-- Name: players; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.players (
    id integer NOT NULL,
    nationid integer NOT NULL,
    game_id integer NOT NULL,
    file_id integer NOT NULL,
    name character varying DEFAULT ''::character varying NOT NULL,
    password character varying,
    startprov integer,
    cdkey bigint
);




--
-- Name: players_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.players_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: players_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.players_id_seq OWNED BY public.players.id;


--
-- Name: subjobs; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.subjobs (
    subjob_id integer NOT NULL,
    job_id integer NOT NULL,
    label text NOT NULL,
    logs text,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    finished_at timestamp with time zone,
    meta text DEFAULT ''::text NOT NULL
);




--
-- Name: subjobs_subjob_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.subjobs_subjob_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: subjobs_subjob_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.subjobs_subjob_id_seq OWNED BY public.subjobs.subjob_id;


--
-- Name: turns; Type: TABLE; Schema: public; Owner: blitzserver_dom6
--

CREATE TABLE public.turns (
    id integer NOT NULL,
    game_id integer NOT NULL,
    turn_number integer NOT NULL,
    file_id integer NOT NULL,
    created_at timestamp without time zone DEFAULT now() NOT NULL,
    archived boolean DEFAULT false NOT NULL,
    turnkey integer,
    winner integer
);




--
-- Name: turns_id_seq; Type: SEQUENCE; Schema: public; Owner: blitzserver_dom6
--

CREATE SEQUENCE public.turns_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;




--
-- Name: turns_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: blitzserver_dom6
--

ALTER SEQUENCE public.turns_id_seq OWNED BY public.turns.id;


--
-- Name: admin_logs id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.admin_logs ALTER COLUMN id SET DEFAULT nextval('public.admin_logs_id_seq'::regclass);


--
-- Name: connections id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.connections ALTER COLUMN id SET DEFAULT nextval('public.connections_id_seq'::regclass);


--
-- Name: disciples id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.disciples ALTER COLUMN id SET DEFAULT nextval('public.disciples_id_seq'::regclass);


--
-- Name: discord_configs id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.discord_configs ALTER COLUMN id SET DEFAULT nextval('public.discord_configs_id_seq'::regclass);


--
-- Name: email_configs id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.email_configs ALTER COLUMN id SET DEFAULT nextval('public.email_configs_id_seq'::regclass);


--
-- Name: files id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.files ALTER COLUMN id SET DEFAULT nextval('public.files_id_seq'::regclass);


--
-- Name: game_logs id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.game_logs ALTER COLUMN id SET DEFAULT nextval('public.game_logs_id_seq'::regclass);


--
-- Name: game_mods id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.game_mods ALTER COLUMN id SET DEFAULT nextval('public.game_mods_id_seq'::regclass);


--
-- Name: games id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.games ALTER COLUMN id SET DEFAULT nextval('public.games_id_seq'::regclass);


--
-- Name: jobs job_id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.jobs ALTER COLUMN job_id SET DEFAULT nextval('public.jobs_job_id_seq'::regclass);


--
-- Name: map_provinces id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.map_provinces ALTER COLUMN id SET DEFAULT nextval('public.map_provinces_id_seq'::regclass);


--
-- Name: maps id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.maps ALTER COLUMN id SET DEFAULT nextval('public.maps_id_seq'::regclass);


--
-- Name: mods id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.mods ALTER COLUMN id SET DEFAULT nextval('public.mods_id_seq'::regclass);


--
-- Name: nations id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.nations ALTER COLUMN id SET DEFAULT nextval('public.nations_id_seq'::regclass);


--
-- Name: player_turns id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.player_turns ALTER COLUMN id SET DEFAULT nextval('public.player_turns_id_seq'::regclass);


--
-- Name: players id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.players ALTER COLUMN id SET DEFAULT nextval('public.players_id_seq'::regclass);


--
-- Name: subjobs subjob_id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.subjobs ALTER COLUMN subjob_id SET DEFAULT nextval('public.subjobs_subjob_id_seq'::regclass);


--
-- Name: turns id; Type: DEFAULT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.turns ALTER COLUMN id SET DEFAULT nextval('public.turns_id_seq'::regclass);


--

--





--

--





--
-- Name: admin_logs admin_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.admin_logs
    ADD CONSTRAINT admin_logs_pkey PRIMARY KEY (id);


--
-- Name: binaries binaries_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.binaries
    ADD CONSTRAINT binaries_pkey PRIMARY KEY (binary_id);


--
-- Name: connections connections_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.connections
    ADD CONSTRAINT connections_pkey PRIMARY KEY (id);


--
-- Name: disciples disciples_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.disciples
    ADD CONSTRAINT disciples_pkey PRIMARY KEY (id);


--
-- Name: discord_configs discord_configs_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.discord_configs
    ADD CONSTRAINT discord_configs_pkey PRIMARY KEY (id);


--
-- Name: email_configs email_configs_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.email_configs
    ADD CONSTRAINT email_configs_pkey PRIMARY KEY (id);


--
-- Name: mods file_id; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.mods
    ADD CONSTRAINT file_id UNIQUE (file_id);


--
-- Name: files files_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.files
    ADD CONSTRAINT files_pkey PRIMARY KEY (id);


--
-- Name: game_logs game_logs_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.game_logs
    ADD CONSTRAINT game_logs_pkey PRIMARY KEY (id);


--
-- Name: game_mods game_mod; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.game_mods
    ADD CONSTRAINT game_mod UNIQUE (game_id, mod_id);


--
-- Name: game_mods game_mods_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.game_mods
    ADD CONSTRAINT game_mods_pkey PRIMARY KEY (id);


--
-- Name: nations gamenation_constraint; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.nations
    ADD CONSTRAINT gamenation_constraint UNIQUE (game_id, nation_id);


--
-- Name: games games_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.games
    ADD CONSTRAINT games_pkey PRIMARY KEY (id);


--
-- Name: jobs jobs_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.jobs
    ADD CONSTRAINT jobs_pkey PRIMARY KEY (job_id);


--
-- Name: map_plane_images map_plane_images_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.map_plane_images
    ADD CONSTRAINT map_plane_images_pkey PRIMARY KEY (map_id, map_file_id, image_file_id);


--
-- Name: map_planes map_planes_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.map_planes
    ADD CONSTRAINT map_planes_pkey PRIMARY KEY (map_id, map_file_id);


--
-- Name: map_provinces map_provinces_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.map_provinces
    ADD CONSTRAINT map_provinces_pkey PRIMARY KEY (id);


--
-- Name: maps_dom6 maps_dom6_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.maps_dom6
    ADD CONSTRAINT maps_dom6_pkey PRIMARY KEY (map_id);


--
-- Name: maps maps_mapfile_id_tgafile_id_winterfile_id_key; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.maps
    ADD CONSTRAINT maps_mapfile_id_tgafile_id_winterfile_id_key UNIQUE (mapfile_id, tgafile_id, winterfile_id);


--
-- Name: maps maps_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.maps
    ADD CONSTRAINT maps_pkey PRIMARY KEY (id);


--
-- Name: mods mods_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.mods
    ADD CONSTRAINT mods_pkey PRIMARY KEY (id);


--
-- Name: disciples nation_game_unique; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.disciples
    ADD CONSTRAINT nation_game_unique UNIQUE (nation_id, game_id);


--
-- Name: players nationid_game_id_unique; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.players
    ADD CONSTRAINT nationid_game_id_unique UNIQUE (nationid, game_id);


--
-- Name: nations nations_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.nations
    ADD CONSTRAINT nations_pkey PRIMARY KEY (id);


--
-- Name: player_turns player_turns_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.player_turns
    ADD CONSTRAINT player_turns_pkey PRIMARY KEY (id);


--
-- Name: players players_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.players
    ADD CONSTRAINT players_pkey PRIMARY KEY (id);


--
-- Name: subjobs subjobs_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.subjobs
    ADD CONSTRAINT subjobs_pkey PRIMARY KEY (subjob_id);


--
-- Name: turns turns_pkey; Type: CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.turns
    ADD CONSTRAINT turns_pkey PRIMARY KEY (id);


--
-- Name: file_filebinary; Type: INDEX; Schema: public; Owner: blitzserver_dom6
--

CREATE UNIQUE INDEX file_filebinary ON public.files USING btree (filename, public.digest(filebinary, 'sha1'::text));


--
-- Name: nation_turn; Type: INDEX; Schema: public; Owner: blitzserver_dom6
--

CREATE UNIQUE INDEX nation_turn ON public.player_turns USING btree (game_id, nation_id, turn_number) WHERE (archived IS FALSE);


--
-- Name: turn_number_game_id; Type: INDEX; Schema: public; Owner: blitzserver_dom6
--

CREATE UNIQUE INDEX turn_number_game_id ON public.turns USING btree (game_id, turn_number) WHERE (archived IS FALSE);


--
-- Name: turns next_turn; Type: TRIGGER; Schema: public; Owner: blitzserver_dom6
--

CREATE TRIGGER next_turn AFTER INSERT ON public.turns FOR EACH ROW EXECUTE FUNCTION public.update_next_turn();


--
-- Name: binaries binaries_binary_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.binaries
    ADD CONSTRAINT binaries_binary_id_fkey FOREIGN KEY (binary_id) REFERENCES public.files(id);


--
-- Name: turns fk_file_file_id; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.turns
    ADD CONSTRAINT fk_file_file_id FOREIGN KEY (file_id) REFERENCES public.files(id);


--
-- Name: player_turns fk_file_trnfile_id; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.player_turns
    ADD CONSTRAINT fk_file_trnfile_id FOREIGN KEY (trnfile_id) REFERENCES public.files(id);


--
-- Name: player_turns fk_file_twohfile_id; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.player_turns
    ADD CONSTRAINT fk_file_twohfile_id FOREIGN KEY (twohfile_id) REFERENCES public.files(id);


--
-- Name: players fk_game; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.players
    ADD CONSTRAINT fk_game FOREIGN KEY (game_id) REFERENCES public.games(id);


--
-- Name: nations fk_game; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.nations
    ADD CONSTRAINT fk_game FOREIGN KEY (game_id) REFERENCES public.games(id);


--
-- Name: email_configs fk_game_email; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.email_configs
    ADD CONSTRAINT fk_game_email FOREIGN KEY (game_id) REFERENCES public.games(id);


--
-- Name: turns fk_game_game_id; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.turns
    ADD CONSTRAINT fk_game_game_id FOREIGN KEY (game_id) REFERENCES public.games(id);


--
-- Name: player_turns fk_game_game_id; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.player_turns
    ADD CONSTRAINT fk_game_game_id FOREIGN KEY (game_id) REFERENCES public.games(id);


--
-- Name: game_mods fk_game_game_id; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.game_mods
    ADD CONSTRAINT fk_game_game_id FOREIGN KEY (game_id) REFERENCES public.games(id);


--
-- Name: connections fk_game_id; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.connections
    ADD CONSTRAINT fk_game_id FOREIGN KEY (game_id) REFERENCES public.games(id);


--
-- Name: maps fk_mapfile; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.maps
    ADD CONSTRAINT fk_mapfile FOREIGN KEY (mapfile_id) REFERENCES public.files(id);


--
-- Name: game_mods fk_mod_mod_id; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.game_mods
    ADD CONSTRAINT fk_mod_mod_id FOREIGN KEY (mod_id) REFERENCES public.mods(id);


--
-- Name: connections fk_nation; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.connections
    ADD CONSTRAINT fk_nation FOREIGN KEY (game_id, nation_id) REFERENCES public.nations(game_id, nation_id);


--
-- Name: player_turns fk_nation_nation_id_game_id; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.player_turns
    ADD CONSTRAINT fk_nation_nation_id_game_id FOREIGN KEY (nation_id, game_id) REFERENCES public.nations(nation_id, game_id);


--
-- Name: maps fk_tgafile; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.maps
    ADD CONSTRAINT fk_tgafile FOREIGN KEY (tgafile_id) REFERENCES public.files(id);


--
-- Name: games games_binary_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.games
    ADD CONSTRAINT games_binary_id_fkey FOREIGN KEY (binary_id) REFERENCES public.binaries(binary_id);


--
-- Name: games games_maps_dom6_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.games
    ADD CONSTRAINT games_maps_dom6_id_fkey FOREIGN KEY (maps_dom6_id) REFERENCES public.maps_dom6(map_id);


--
-- Name: subjobs job_id; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.subjobs
    ADD CONSTRAINT job_id FOREIGN KEY (job_id) REFERENCES public.jobs(job_id);


--
-- Name: map_plane_images map_plane_images_image_file_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.map_plane_images
    ADD CONSTRAINT map_plane_images_image_file_id_fkey FOREIGN KEY (image_file_id) REFERENCES public.files(id);


--
-- Name: map_plane_images map_plane_images_map_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.map_plane_images
    ADD CONSTRAINT map_plane_images_map_id_fkey FOREIGN KEY (map_id) REFERENCES public.maps_dom6(map_id);


--
-- Name: map_plane_images map_plane_images_map_plane_fkey; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.map_plane_images
    ADD CONSTRAINT map_plane_images_map_plane_fkey FOREIGN KEY (map_id, map_file_id) REFERENCES public.map_planes(map_id, map_file_id);


--
-- Name: map_planes map_planes_map_file_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.map_planes
    ADD CONSTRAINT map_planes_map_file_id_fkey FOREIGN KEY (map_file_id) REFERENCES public.files(id);


--
-- Name: map_planes map_planes_map_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.map_planes
    ADD CONSTRAINT map_planes_map_id_fkey FOREIGN KEY (map_id) REFERENCES public.maps_dom6(map_id);


--
-- Name: players players_file_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: blitzserver_dom6
--

ALTER TABLE ONLY public.players
    ADD CONSTRAINT players_file_id_fkey FOREIGN KEY (file_id) REFERENCES public.files(id);


--
-- Name: SCHEMA public; Type: ACL; Schema: -; Owner: pg_database_owner
--

REVOKE USAGE ON SCHEMA public FROM PUBLIC;
GRANT ALL ON SCHEMA public TO PUBLIC;


--
-- PostgreSQL database dump complete
--

