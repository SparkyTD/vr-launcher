export const Api = {
    ListGames: "http://192.168.1.108:3001/api/games",
    GetGame: (id: string) => `${Api.ListGames}/${id}`,
    GetGameCover: (id: string) => `${Api.ListGames}/${id}/cover`,
    StartGame: (id: string) => `${Api.ListGames}/${id}/launch`,
    GetActiveGame: "http://192.168.1.108:3001/api/games/active",
    KillActiveGame: "http://192.168.1.108:3001/api/games/active/kill",
    ListAudioInputs: "http://192.168.1.108:3001/api/audio/inputs",
    ListAudioOutputs: "http://192.168.1.108:3001/api/audio/outputs",
    SetDefaultAudioInput: (id: number) => `${Api.ListAudioInputs}/${id}/default`,
    SetDefaultAudioOutput: (id: number) => `${Api.ListAudioOutputs}/${id}/default`,
    StateSock: "ws://192.168.1.108:3001/api/sock",
    DeviceGetBatteryInfo: "http://192.168.1.108:3001/api/device/battery",

    DebugGetAgent: "http://192.168.1.108:3001/api/debug/agent"
}

const Api2 = {
    BaseUrl: (proto: string) => `${proto}://192.168.1.108:3001/api`,

    // Game APIs
    ListGames: () => `${Api2.BaseUrl("http")}/games`,
    GetGameCover: (id: string) => `${Api2.ListGames()}/${id}/cover`,
    StartGame: (id: string) => `${Api2.ListGames()}/${id}/launch`,
    GetActiveGame: () => `${Api2.ListGames()}/active`,
    KillActiveGame: () => `${Api2.ListGames()}/active/kill`,

    // Audio APIs
    ListAudioInputs: () => `${Api2.BaseUrl("http")}/audio/inputs`,
    ListAudioOutputs: () => `${Api2.BaseUrl("http")}/audio/outputs`,
    SetDefaultAudioInput: (id: number) => `${Api2.ListAudioInputs}/${id}/default`,
    SetDefaultAudioOutput: (id: number) => `${Api2.ListAudioOutputs}/${id}/default`,

    // Socket endpoint
    StateSock: () => `${Api2.BaseUrl("ws")}/sock`,

    // Device status
    DeviceGetBatteryInfo: () => `${Api2.BaseUrl("http")}/device/battery`,
}