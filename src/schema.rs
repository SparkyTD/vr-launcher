// @generated automatically by Diesel CLI.

diesel::table! {
    games (id) {
        id -> Text,
        title -> Text,
        cover -> Nullable<Binary>,
        vr_backend -> Text,
        vr_backend_args -> Text,
        pressure_vessel -> Bool,
        steam_app_id -> Nullable<BigInt>,
        command_line -> Nullable<Text>,
        proton_version -> Nullable<Text>,
        use_overlay -> Bool,
    }
}
