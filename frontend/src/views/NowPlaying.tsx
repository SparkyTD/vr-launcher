import {GameInfo} from "../components/GameCard.tsx";
import {createSignal, onCleanup} from "solid-js";
import {ClockIcon, RefreshCwIcon, XIcon} from "lucide-solid";
import {Api} from "../api.ts";
import {GameSession} from "../rust_bindings.ts";
import {useWebSocket} from "../socket.ts";

export type NowPlayingProps = {
    game: GameInfo,
    session: GameSession,
    onBack: () => void,
}

export default function NowPlaying({game, session, onBack}: NowPlayingProps) {
    const [sessionTime, setSessionTime] = createSignal(getSessionTime());

    // Session timer
    const timer = setInterval(() => {
        setSessionTime(getSessionTime());
    }, 1000);
    onCleanup(() => clearInterval(timer));

    function getSessionTime() {
        let now = Date.now() / 1000;
        let start = Number(session.startTimeEpoch);
        return Math.floor(now - start);
    }

    useWebSocket(data => {
        let parts = (data as string).split(':');
        let command = parts[0];

        if (command == "inactive") {
            onBack();
        }
    });

    const formatTime = (seconds: number) => {
        const hours = Math.floor(seconds / 3600);
        const mins = Math.floor((seconds % 3600) / 60);
        const secs = seconds % 60;

        if (hours > 0) {
            return `${hours}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
        }
        return `${mins}:${secs.toString().padStart(2, '0')}`;
    };

    const handleKillGame = Api.KillActiveGameAsync;

    return (
        <div class="fixed inset-0 z-10">
            {/* Background Image */}
            <img
                alt={game.title}
                src={game.cover}
                class="absolute inset-0 w-full h-full object-cover blur-xl opacity-50"
            />

            {/* Gradient Overlays */}
            <div class="absolute inset-0 bg-gradient-to-b from-black/60 via-transparent to-black/80"></div>
            <div class="absolute inset-0 bg-gradient-to-r from-black/80 via-transparent to-transparent"></div>

            {/* Content Container */}
            <div class="absolute inset-0 flex flex-col justify-end p-16 pb-24">
                <div class="max-w-4xl">
                    {/* Game Title */}
                    <h1 class="text-8xl font-bold mb-6 leading-none drop-shadow-2xl">
                        {game.title}
                    </h1>

                    {/* Game Info */}
                    <div class="flex items-center space-x-8 mb-8 ms-2 text-lg text-white/90">
                        <div class="flex items-center space-x-2">
                            <div class="w-3 h-3 bg-green-500 rounded-full animate-pulse"></div>
                            <span class="font-medium">Now Playing</span>
                        </div>

                        <div class="flex items-center space-x-2">
                            <ClockIcon class="w-5 h-5"/>
                            <span>Session: {formatTime(sessionTime())}</span>
                        </div>
                    </div>

                    {/* Action Buttons */}
                    <div class="flex items-center space-x-4 ms-1">
                        <button
                            onclick={handleKillGame}
                            class="backdrop-blur-md bg-red-600/20 rounded-xl p-4 pe-6 border border-red-600/10 flex items-center gap-3 cursor-pointer hover:bg-red-400/20 transition-all">
                            <XIcon width={24} height={24} />
                            <span>Close game</span>
                        </button>
                        <button
                            onclick={Api.ReconnectBackendAsync}
                            class="backdrop-blur-md bg-gray-500/20 rounded-xl p-4 pe-6 border border-gray-500/10 flex items-center gap-3 cursor-pointer hover:bg-gray-400/20 transition-all">
                            <RefreshCwIcon width={20} height={20}/>
                            <span>Reconnect backend</span>
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}