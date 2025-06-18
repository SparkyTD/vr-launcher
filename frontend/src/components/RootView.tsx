import Header from "./Header.tsx";
import GamesGrid from "./GamesGrid.tsx";
import {GameInfo} from "./GameCard.tsx";
import {createResource, createSignal, Show} from "solid-js";
import NowPlaying from "./NowPlaying.tsx";
import {Api} from "../api.ts";
import {GameSession} from "../rust_bindings.ts";
import {useWebSocket} from "../socket.ts";
import Modal, {ErrorModalContents} from "./Modal.tsx";
import {v4 as uuidv4} from 'uuid';

type AppState = 'grid' | 'playing';

export default function RootView() {
    const [currentState, setCurrentState] = createSignal<AppState>('grid');
    const [selectedGame, setSelectedGame] = createSignal<GameInfo | null>(null);
    const [activeSession, setActiveSession] = createSignal<GameSession | null>(null);
    const [errorModelContent, setErrorModelContent] = createSignal<ErrorModalContents | null>(null);

    const handleGameClicked = async (game: GameInfo) => {
        let idemToken = uuidv4();
        let result = await fetch(Api.StartGame(game.id) + `?idem_token=${idemToken}`, {
            method: "POST",
        });

        if (result.status >= 500) {
            setErrorModelContent({
                title: "Failed to launch instance",
                text: await result.text(),
            });
        }
    };

    const handleBackToGrid = () => {
        setCurrentState('grid');
        setSelectedGame(null);
        setActiveSession(null);
    };

    const [games] = createResource(async () => {
        const response = (await fetch(Api.ListGames)
            .then((res) => res.json()))
            .map((game: any) => {
                return {
                    id: game.id,
                    title: game.title,
                    cover: Api.GetGameCover(game.id),
                    playtimeSeconds: game.playtime_sec,
                } as GameInfo;
            });

        return response as GameInfo[];
    });

    createResource(games, async (gamesList) => {
        if (!gamesList) return null;

        let result = (await fetch(Api.GetActiveGame)
            .then(res => res.json())
            .then(json => json as GameSession));

        console.log(`Active game: ${result.game.title}`);

        if (!!result.game.id) {
            let game_info = gamesList.find((g) => g.id === result.game.id);
            if (game_info) {
                setCurrentState('playing');
                setSelectedGame(game_info);
                setActiveSession(result);
            }
        } else {
            console.warn("Active game not found!")
        }

        return result;
    });

    useWebSocket(data => {
        let parts = (data as string).split(':');
        let command = parts[0];
        let args = parts.slice(1).join(':');

        if (command == "active") {
            let session = JSON.parse(args) as GameSession
            let game_info = games()!.find((g) => g.id === session.game.id);
            setCurrentState('playing');
            setSelectedGame(game_info!);
            setActiveSession(session);
        }
    });

    return <main class="min-h-screen bg-gradient-to-br p-4 from-gray-900 via-black to-gray-900 text-white overflow-hidden">
        <div class="fixed inset-0 bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-purple-900/20 via-transparent to-transparent pointer-events-none"/>
        <div class="fixed inset-0 bg-[radial-gradient(ellipse_at_bottom_right,_var(--tw-gradient-stops))] from-blue-900/20 via-transparent to-transparent pointer-events-none"/>

        <Header/>

        <Show when={currentState() === 'grid'}>
            <GamesGrid games={games} gameClicked={handleGameClicked}/>
        </Show>

        <Show when={currentState() === 'playing' && selectedGame() && activeSession()}>
            <NowPlaying game={selectedGame()!} onBack={handleBackToGrid} session={activeSession()!}/>
        </Show>

        <Modal contents={errorModelContent()} onClose={() => setErrorModelContent(null)}/>
    </main>;
}