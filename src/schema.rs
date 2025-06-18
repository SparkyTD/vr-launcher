// @generated automatically by Diesel CLI.

diesel::table! {
    games (id) {
        id -> Text,
        title -> Text,
        cover -> Nullable<Binary>,
        vr_backend -> Text,
        steam_app_id -> Nullable<BigInt>,
        command_line -> Nullable<Text>,
        total_playtime_sec -> Integer,
        proton_version -> Nullable<Text>,
    }
}
