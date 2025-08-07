import {createResource, createSignal, Show} from "solid-js";
import {Api} from "../api.ts";
import {AndroidBatteryInfo} from "../rust_bindings.ts";
// @ts-ignore
import {clickOutside} from "./utils/clickOutside.ts";
import {BatteryFullIcon, BatteryLowIcon, BatteryMediumIcon, BatteryWarningIcon, ChevronDown, CircleAlertIcon} from "lucide-solid";
import AreaChart from "./AreaChart.tsx";
import {useWebSocket} from "../socket.ts";

export default function BatteryIndicator() {
    const [isOpen, setIsOpen] = createSignal(false);
    const [batteryInfo, setBatteryInfo] = createSignal<AndroidBatteryInfo | null>(null);

    createResource(async () => {
        setBatteryInfo(await Api.GetDeviceBatteryInfo());
    });

    useWebSocket(data => {
        let parts = (data as string).split(':');
        let command = parts[0];
        let args = parts.slice(1).join(':');

        if (command == "battery") {
            setBatteryInfo(JSON.parse(args) as AndroidBatteryInfo);
        }
    });

    return <Show when={!!batteryInfo()}>
        {/*@ts-ignore*/}
        <div class="relative" use:clickOutside={() => setIsOpen(false)}>
            <div class="backdrop-blur-md bg-white/5 rounded-xl px-4 py-3 pe-5 border border-white/10 flex items-center gap-3 hover:bg-white/10 transition-all cursor-pointer"
                 onClick={() => setIsOpen(!isOpen())}>
                {(() => {
                    let level = batteryInfo()!.stats.level;
                    return level >= 75
                        ? <BatteryFullIcon class="text-green-500"/>
                        : level >= 45
                            ? <BatteryMediumIcon class="text-yellow-500"/>
                            : level >= 15
                                ? <BatteryLowIcon class="text-orange-500"/>
                                : <BatteryWarningIcon class="text-red-500"/>
                })()}
                <div>
                    <span class="bold">{batteryInfo()!.stats.level}%</span>
                    <span>&nbsp;{batteryInfo()!.stats.powerSource}</span>
                </div>

                {batteryInfo()!.stats.isWeakCharger && <CircleAlertIcon class="text-orange-500 w-5"/>}

                <ChevronDown class={`w-4 h-4 text-zinc-400 transition-transform duration-200 ${isOpen() ? "rotate-180" : ""}`}/>
            </div>

            <Show when={isOpen()}>
                <div class="absolute right-0 mt-2 w-80 rounded-xl backdrop-blur-xl bg-zinc-900/5 border border-white/10 shadow-xl z-50 p-4">
                    <div class="space-y-3 text-sm">
                        <div class="flex justify-between items-center">
                            <span class="text-zinc-400">Battery Level</span>
                            <span class="font-medium">{batteryInfo()!.stats.level}%</span>
                        </div>

                        <div class="flex justify-between items-center">
                            <span class="text-zinc-400">Power Source</span>
                            <span class="font-medium">
                                {batteryInfo()!.stats.powerSource}
                                {batteryInfo()!.stats.isWeakCharger && " (weak)"}
                            </span>
                        </div>

                        <div class="flex justify-between items-center">
                            <span class="text-zinc-400">Max Charging Power</span>
                            <span class="font-medium">
                                {((batteryInfo()!.stats.maxChargeCurrentMa / 1000) * (batteryInfo()!.stats.maxChargeVoltageMv / 1000)).toFixed(1)}W
                            </span>
                        </div>

                        <div class="flex justify-between items-center">
                            <span class="text-zinc-400">Temperature</span>
                            <span class="font-medium">{(batteryInfo()!.stats.temperature / 10).toFixed(1)}°C</span>
                        </div>

                        <div class="flex justify-between items-center">
                            <span class="text-zinc-400">Status</span>
                            <span class="font-medium">{batteryInfo()!.stats.status}</span>
                        </div>

                        <div class="flex justify-between items-center">
                            <span class="text-zinc-400">Health</span>
                            <span class="font-medium">{batteryInfo()!.stats.health}</span>
                        </div>
                    </div>

                    <div class="absolute top-0 left-0 bottom-0 right-0">
                        <AreaChart
                            color='rgba(128, 255, 128, 0.2)'
                            data={batteryInfo()!.history}
                            fillParent={true}/>
                    </div>
                </div>
            </Show>
        </div>
    </Show>
}