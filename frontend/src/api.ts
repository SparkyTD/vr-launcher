import {GameInfo} from "./components/GameCard.tsx";
import {AndroidBatteryInfo, GameSession} from "./rust_bindings.ts";
import {AudioDevice} from "./components/AudioSelector.tsx";

export class Api {
    public static async ListGamesAsync(): Promise<GameInfo[]> {
        return (await fetch(Api.GetApiUrl("/games"))
            .then((res) => res.json()))
            .map((game: any) => {
                return {
                    id: game.id,
                    title: game.title,
                    cover: Api.GetApiUrl(`/games/${game.id}/cover`),
                    playtimeSeconds: game.playtime_sec,
                } as GameInfo;
            });
    }

    public static async StartGameAsync(game: GameInfo, token: string): Promise<Response> {
        return await fetch(Api.GetApiUrl(`/games/${game.id}/launch`) + `?idem_token=${token}`, {
            method: "POST",
        });
    }

    public static async GetActiveSessionAsync(): Promise<GameSession> {
        return await fetch(Api.GetApiUrl("/games/active"))
            .then(res => res.json())
            .then(json => json as GameSession);
    }

    public static async KillActiveGameAsync(): Promise<void> {
        await fetch(Api.GetApiUrl(`/games/active/kill`), {
            method: "POST",
        })
    }

    public static async ReconnectBackendAsync(): Promise<void> {
        await fetch(Api.GetApiUrl(`/games/reload_backend`), {
            method: "POST",
        })
    }

    public static async ListAudioInputsAsync(): Promise<AudioDevice[]> {
        return await fetch(Api.GetApiUrl("/audio/inputs"))
            .then(res => res.json())
            .then(j => j as AudioDevice[]);
    }

    public static async ListAudioOutputsAsync(): Promise<AudioDevice[]> {
        return await fetch(Api.GetApiUrl("/audio/outputs"))
            .then(res => res.json())
            .then(j => j as AudioDevice[]);
    }

    public static async SetDefaultAudioInputAsync(device: AudioDevice): Promise<void> {
        await fetch(Api.GetApiUrl(`/audio/inputs/${device.id}/default`), {
            method: "POST",
        });
    }

    public static async SetDefaultAudioOutputAsync(device: AudioDevice): Promise<void> {
        await fetch(Api.GetApiUrl(`/audio/outputs/${device.id}/default`), {
            method: "POST",
        });
    }

    public static async GetDeviceBatteryInfo(): Promise<AndroidBatteryInfo> {
        return await fetch(Api.GetApiUrl("/device/battery"))
            .then(res => res.json())
            .then(res => res as AndroidBatteryInfo);
    }

    public static GetSockUrl(): string {
        return Api.GetApiUrl("/sock", "ws");
    }

    static GetApiUrl(path: string, proto: string = "http"): string {
        path = path.replace(/^\//g, '');
        return `${proto}://192.168.1.108:3001/api/${path}`;
    }
}