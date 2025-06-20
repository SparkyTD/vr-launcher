import {PlayIcon} from "lucide-solid";
import {createSignal, Show} from "solid-js";

export type GameInfo = {
    id: string;
    title: string;
    cover: string;
    playtimeSeconds: number;
}

export type GameProps = {
    game: GameInfo;
    clicked?: (game: GameInfo) => Promise<void>;
}

export default function GameCard({game, clicked}: GameProps) {
    const [isLoading, setIsLoading] = createSignal(false);

    async function handleClick() {
        if (!clicked) {
            return;
        }

        setIsLoading(true);
        await clicked(game);
        setIsLoading(false);
    }

    return <div class="group relative overflow-hidden rounded-3xl backdrop-blur-xl bg-white/5 border border-white/10 hover:bg-white/10 transition-all duration-300 hover:scale-[1.02] hover:shadow-2xl cursor-pointer"
                on:click={handleClick}>
        <div class="aspect-[3/4] relative overflow-hidden">
            <img
                src={game.cover}
                alt={game.title}
                class="w-full h-full object-cover group-hover:scale-110 transition-transform duration-500"
            />
            <div class="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-transparent"/>

            {/* Play button overlay */}
            <div class="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity duration-300">
                <Show when={!isLoading()}>
                    <PlayIcon width={60} height={60}/>
                </Show>
            </div>

            {/* Loading spinner */}
            <Show when={isLoading()}>
                <div class="absolute inset-0 h-full bg-black/50">
                    <div class="flex h-full items-center justify-center">
                        <div
                            class="inline-block h-14 w-14 animate-spin rounded-full border-4 border-solid border-current border-e-transparent align-[-0.125em] text-surface motion-reduce:animate-[spin_1.5s_linear_infinite] dark:text-white"
                            role="status">
                            <span class="!absolute !-m-px !h-px !w-px !overflow-hidden !whitespace-nowrap !border-0 !p-0 ![clip:rect(0,0,0,0)]"/>
                        </div>
                    </div>
                </div>
            </Show>

        </div>
    </div>
}