CREATE TABLE admin_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    datetime DATETIME NOT NULL,
    action TEXT NOT NULL
);

CREATE TABLE binaries (
    binary_id INTEGER PRIMARY KEY,
    executable_path_linux TEXT NOT NULL,
    executable_path_windows TEXT NOT NULL,
    version TEXT NOT NULL,
    version_number INTEGER NOT NULL,
    foldered_maps BOOLEAN NOT NULL
);

CREATE TABLE connections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    connected DATETIME NOT NULL,
    disconnected DATETIME,
    game_id INTEGER,
    nation_id INTEGER,
    ip TEXT
);

CREATE TABLE disciples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    nation_id INTEGER NOT NULL,
    is_disciple INTEGER NOT NULL,
    team INTEGER
);

CREATE TABLE discord_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    last_turn_notified INTEGER,
    discord_guildid TEXT NOT NULL,
    discord_channelid TEXT NOT NULL,
    message TEXT NOT NULL,
    hours_remaining INTEGER
);

CREATE TABLE email_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nation_id INTEGER NOT NULL,
    game_id INTEGER NOT NULL,
    hours_before_host INTEGER NOT NULL,
    email_address TEXT NOT NULL,
    last_turn_notified INTEGER,
    subject TEXT NOT NULL DEFAULT 'Default email notifcation subject, you probably have a new turn',
    body TEXT NOT NULL DEFAULT 'Default email notification body, you probably have a new turn',
    is_reminder BOOLEAN NOT NULL DEFAULT 0
);

CREATE TABLE files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    filename TEXT NOT NULL,
    filebinary BLOB NOT NULL
);

CREATE TABLE game_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    datetime DATETIME NOT NULL,
    turn_number INTEGER NOT NULL,
    output TEXT NOT NULL,
    error TEXT NOT NULL,
    log_command TEXT NOT NULL DEFAULT ''
);

CREATE TABLE game_mods (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    mod_id INTEGER NOT NULL
);

CREATE TABLE games (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    era INTEGER NOT NULL,
    map_id INTEGER NOT NULL DEFAULT 0,
    port INTEGER,
    timer INTEGER,
    thrones_t1 INTEGER NOT NULL DEFAULT 5,
    thrones_t2 INTEGER NOT NULL DEFAULT 0,
    thrones_t3 INTEGER NOT NULL DEFAULT 0,
    throne_points_required INTEGER NOT NULL DEFAULT 5,
    research_diff INTEGER NOT NULL DEFAULT 2,
    research_rand BOOLEAN NOT NULL DEFAULT 1,
    hof_size INTEGER NOT NULL DEFAULT 10,
    global_size INTEGER NOT NULL DEFAULT 5,
    indepstr INTEGER NOT NULL DEFAULT 4,
    magicsites INTEGER NOT NULL DEFAULT 40,
    eventrarity INTEGER NOT NULL DEFAULT 2,
    richness INTEGER NOT NULL DEFAULT 100,
    resources INTEGER NOT NULL DEFAULT 100,
    recruitment INTEGER NOT NULL DEFAULT 100,
    supplies INTEGER NOT NULL DEFAULT 100,
    startprov INTEGER NOT NULL DEFAULT 2,
    renaming BOOLEAN NOT NULL DEFAULT 0,
    scoregraphs BOOLEAN NOT NULL DEFAULT 1,
    nationinfo BOOLEAN NOT NULL DEFAULT 1,
    artrest BOOLEAN NOT NULL DEFAULT 1,
    teamgame BOOLEAN NOT NULL DEFAULT 0,
    clustered BOOLEAN NOT NULL DEFAULT 0,
    storyevents INTEGER NOT NULL DEFAULT 1,
    newailvl INTEGER NOT NULL DEFAULT 2,
    next_turn DATETIME,
    password TEXT NOT NULL DEFAULT 'password',
    archived BOOLEAN NOT NULL DEFAULT 0,
    masterpass TEXT,
    private BOOLEAN NOT NULL DEFAULT 0,
    last_activity DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    require_password BOOLEAN NOT NULL DEFAULT 0,
    select_start INTEGER NOT NULL DEFAULT 0,
    modified_mapcrc INTEGER,
    cataclysm INTEGER,
    max_stales INTEGER,
    motd TEXT,
    conq_all BOOLEAN NOT NULL DEFAULT 0,
    binary_id INTEGER,
    maps_dom6_id INTEGER,
    formal_diplo INTEGER NOT NULL DEFAULT 0,
    research_9_slow BOOLEAN NOT NULL DEFAULT 0,
    mercslots INTEGER NOT NULL DEFAULT 5,
    cheat_detection BOOLEAN NOT NULL DEFAULT 1,
    provs_per_player INTEGER NOT NULL DEFAULT 15,
    randommap_wrapping INTEGER NOT NULL DEFAULT 0,
    log_level INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE jobs (
    job_id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    turn_number INTEGER NOT NULL,
    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reaped BOOLEAN NOT NULL DEFAULT 0,
    finished_at DATETIME
);

CREATE TABLE map_plane_images (
    map_id INTEGER NOT NULL,
    map_file_id INTEGER NOT NULL,
    image_file_id INTEGER NOT NULL,
    hash INTEGER NOT NULL,
    PRIMARY KEY (map_id, map_file_id, image_file_id)
);

CREATE TABLE map_planes (
    map_id INTEGER NOT NULL,
    map_file_id INTEGER NOT NULL,
    hash INTEGER NOT NULL,
    prov_count INTEGER,
    PRIMARY KEY (map_id, map_file_id)
);

CREATE TABLE map_provinces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    map_id INTEGER NOT NULL,
    province_id INTEGER NOT NULL,
    x INTEGER NOT NULL,
    y INTEGER NOT NULL,
    unit_x REAL NOT NULL,
    unit_y REAL NOT NULL
);

CREATE TABLE maps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    mapfile_id INTEGER NOT NULL,
    tgafile_id INTEGER NOT NULL,
    winterfile_id INTEGER,
    province_count INTEGER NOT NULL DEFAULT 0,
    uw_count INTEGER NOT NULL DEFAULT 0,
    archived BOOLEAN NOT NULL DEFAULT 0,
    map_crc INTEGER,
    tga_crc INTEGER,
    winter_crc INTEGER,
    min_version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE maps_dom6 (
    map_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    is_randommap BOOLEAN NOT NULL DEFAULT 0
);

CREATE TABLE mods (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dm_filename TEXT NOT NULL,
    name TEXT NOT NULL,
    file_id INTEGER NOT NULL,
    icon_file_id INTEGER,
    crc INTEGER
);

CREATE TABLE nations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    nation_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    epithet TEXT NOT NULL,
    filename TEXT NOT NULL DEFAULT 'unknown'
);

CREATE TABLE player_turns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    turn_number INTEGER NOT NULL,
    nation_id INTEGER NOT NULL,
    game_id INTEGER NOT NULL,
    trnfile_id INTEGER,
    twohfile_id INTEGER,
    archived BOOLEAN NOT NULL DEFAULT 0,
    status INTEGER NOT NULL DEFAULT 0,
    twoh_crc INTEGER
);

CREATE TABLE players (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nationid INTEGER NOT NULL,
    game_id INTEGER NOT NULL,
    file_id INTEGER NOT NULL,
    name TEXT NOT NULL DEFAULT '',
    password TEXT,
    startprov INTEGER,
    cdkey INTEGER
);

CREATE TABLE subjobs (
    subjob_id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id INTEGER NOT NULL,
    label TEXT NOT NULL,
    logs TEXT,
    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at DATETIME,
    meta TEXT NOT NULL DEFAULT ''
);

CREATE TABLE turns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    turn_number INTEGER NOT NULL,
    file_id INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    archived BOOLEAN NOT NULL DEFAULT 0,
    turnkey INTEGER,
    winner INTEGER
);

-- Unique constraints and indexes
CREATE UNIQUE INDEX idx_mods_file_id ON mods(file_id);
CREATE UNIQUE INDEX idx_game_mods_game_mod ON game_mods(game_id, mod_id);
CREATE UNIQUE INDEX idx_nations_gamenation ON nations(game_id, nation_id);
CREATE UNIQUE INDEX idx_maps_mapfile_tga_winter ON maps(mapfile_id, tgafile_id, winterfile_id);
CREATE UNIQUE INDEX idx_disciples_nation_game ON disciples(nation_id, game_id);
CREATE UNIQUE INDEX idx_players_nationid_game ON players(nationid, game_id);

-- Partial unique indexes (WHERE clause)
CREATE UNIQUE INDEX idx_player_turns_nation_turn ON player_turns(game_id, nation_id, turn_number) WHERE archived = 0;
CREATE UNIQUE INDEX idx_turns_game_turn ON turns(game_id, turn_number) WHERE archived = 0;
