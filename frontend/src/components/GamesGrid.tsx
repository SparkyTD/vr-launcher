import GameCard, {GameInfo} from "./GameCard.tsx";
import {For, Resource} from "solid-js";

export type GamesGridProps = {
    games: Resource<GameInfo[]>;
    gameClicked?: (game: GameInfo) => Promise<void>;
}

export default function GamesGrid({games, gameClicked}: GamesGridProps) {
    return <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-6">
        <For each={games()}>{game => <GameCard game={game} clicked={gameClicked}/>}</For>
    </div>
}