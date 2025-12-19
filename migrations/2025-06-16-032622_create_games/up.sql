-- Your SQL goes here
CREATE TABLE games
(
    id                 TEXT    NOT NULL PRIMARY KEY,
    title              TEXT    NOT NULL,
    cover              BLOB,
    vr_backend         TEXT    NOT NULL,
    vr_backend_args    TEXT    NOT NULL,
    pressure_vessel    INTEGER NOT NULL,
    steam_app_id       BIGINT,
    command_line       TEXT,
    proton_version     TEXT
);

