import {PlayIcon} from "lucide-solid";

export type GameInfo = {
    id: string;
    title: string;
    cover: string;
    playtimeSeconds: number;
}

export type GameProps = {
    game: GameInfo;
    clicked?: (game: GameInfo) => void;
}

export default function GameCard({game, clicked}: GameProps) {
    return <div class="group relative overflow-hidden rounded-3xl backdrop-blur-xl bg-white/5 border border-white/10 hover:bg-white/10 transition-all duration-300 hover:scale-[1.02] hover:shadow-2xl cursor-pointer"
                on:click={() => !!clicked ? clicked!(game) : null}>
        <div class="aspect-[3/4] relative overflow-hidden">
            <img
                src={game.cover}
                alt={game.title}
                class="w-full h-full object-cover group-hover:scale-110 transition-transform duration-500"
            />
            <div class="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-transparent"/>

            {/* Play button overlay */}
            <div class="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-300">
                <PlayIcon width={60} height={60}/>
            </div>
        </div>
    </div>
}