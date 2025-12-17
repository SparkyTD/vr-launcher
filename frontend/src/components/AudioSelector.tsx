import {ChevronDown, MicIcon, Volume2Icon} from "lucide-solid";
import {createResource, createSignal, For, Show} from "solid-js";
import {Api} from "../api.ts";
// @ts-ignore
import {clickOutside} from "./utils/clickOutside.ts";
import {useWebSocket} from "../socket.ts";
import {Observable} from "../utils/observable.ts";

export default function AudioSelector() {
    const [isOpen, setIsOpen] = createSignal(false)

    const [defaultDevices, {refetch}] = createResource(async () => {
        let inputs = await Api.ListAudioInputsAsync();
        let outputs = await Api.ListAudioOutputsAsync();

        return {
            input: inputs.find(i => i.is_default),
            output: outputs.find(i => i.is_default),
        };
    });

    const deviceInfoObservable = new Observable<AudioDevice | null>(null);

    useWebSocket(data => {
        let parts = (data as string).split(':');
        let command = parts[0];

        if (command == "default_output_changed" || command == "default_input_changed") {
            refetch()
        } else if (command == "volume_mute_changed") {
            deviceInfoObservable.value = JSON.parse(parts.slice(1).join(":")) as AudioDevice;
        }
    });

    // @ts-ignore
    return <div class="relative" use:clickOutside={() => setIsOpen(false)}>
        <div class="backdrop-blur-md bg-white/5 rounded-xl px-4 py-3 border border-white/10 flex items-center gap-3 cursor-pointer hover:bg-white/10 transition-all"
             on:click={() => setIsOpen(!isOpen())}>
            <div class="flex items-center gap-2">
                <Volume2Icon class="w-5 h-5 text-blue-400"/>
                <span class="text-sm">{truncateMiddle(defaultDevices()?.output?.description)}</span>
            </div>
            <div class="w-px h-6 bg-white/10"/>
            <div class="flex items-center gap-2">
                <MicIcon class="w-5 h-5 text-blue-400"/>
                <span class="text-sm">{truncateMiddle(defaultDevices()?.input?.description)}</span>
            </div>
            <ChevronDown class={`w-4 h-4 text-zinc-400 transition-transform duration-200 ${isOpen() ? "rotate-180" : ""}`}/>
        </div>

        <Show when={isOpen()}>
            <div class="absolute right-0 mt-2 w-80 max-h-[calc(100vh-5rem)] overflow-y-auto rounded-xl backdrop-blur-xl bg-zinc-900/5 border border-white/10 shadow-xl z-50 p-4">
                <EndpointSelector name="Speakers" type="output" forceRefetch={refetch} deviceVolumeObservable={deviceInfoObservable}/>
                <EndpointSelector name="Microphones" type="input" forceRefetch={refetch} deviceVolumeObservable={deviceInfoObservable}/>
            </div>
        </Show>
    </div>
}

type AudioSelectorProps = {
    name: string
    type: 'input' | 'output',
    forceRefetch: () => void,
    deviceVolumeObservable: Observable<AudioDevice | null>
}

export type AudioDevice = {
    id: number,
    name: string,
    description: string,
    is_default: boolean,
    volume: number,
    is_muted: boolean,
}

function EndpointSelector({name, type, forceRefetch, deviceVolumeObservable}: AudioSelectorProps) {
    const [isLoading, setIsLoading] = createSignal(false);

    const [devices, {refetch}] = createResource(async () => {
        return type == 'input'
            ? await Api.ListAudioInputsAsync()
            : await Api.ListAudioOutputsAsync();
    });

    return <div class={type == "input" ? "relative mt-4" : "relative"}>
        <Show when={isLoading()}>
            <div class="absolute inset-0 bg-black/10 z-10 rounded-lg"></div>
        </Show>

        <h3 class="text-sm font-medium text-zinc-300 mb-2">{name}</h3>
        <div class="space-y-1">
            <For each={devices()}>{device => {
                const [currentVolume, setCurrentVolume] = createSignal(device.volume);
                deviceVolumeObservable.subscribe((info) => {
                    if(info?.id != device.id) {
                        return;
                    }
                    device.volume = info.volume;
                    setCurrentVolume(device.volume);
                })
                return <div class={`${device.is_default ? "flex flex-col" : "flex items-center"} gap-2 p-2 rounded-lg cursor-pointer ${device.is_default ? "bg-white/10" : "hover:bg-white/5"}`}
                            onClick={async () => {
                                setIsLoading(true);
                                try {
                                    type == 'input'
                                        ? await Api.SetDefaultAudioInputAsync(device)
                                        : await Api.SetDefaultAudioOutputAsync(device);
                                    forceRefetch();
                                    await refetch();
                                } catch (error) {
                                    console.error("Failed to update the default audio device", error);
                                }
                                setIsLoading(false);
                            }}>
                    <div class="flex items-center gap-2">
                        {type == 'output' && <Volume2Icon class={`w-5 h-5 me-2 flex-shrink-0 ${device.is_default ? "text-blue-400" : "text-zinc-400"}`}/>}
                        {type == 'input' && <MicIcon class={`w-5 h-5 me-2 flex-shrink-0 ${device.is_default ? "text-blue-400" : "text-zinc-400"}`}/>}
                        <span class={`text-sm flex-1 whitespace-nowrap ${device.is_default ? "text-white" : "text-zinc-300"}`} title={device.description}>
                            {truncateMiddle(device.description)}
                        </span>
                    </div>
                    <Show when={device.is_default}>
                        <div class="w-full mt-2">
                            <input
                                type="range"
                                min="0"
                                max="100"
                                value={currentVolume()}
                                class="w-full h-2 appearance-none cursor-pointer rounded-full bg-white/10 backdrop-blur-sm border border-white/5
                                       [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:h-4
                                       [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-blue-400 [&::-webkit-slider-thumb]:border-2
                                       [&::-webkit-slider-thumb]:border-white/20 [&::-webkit-slider-thumb]:shadow-lg [&::-webkit-slider-thumb]:cursor-pointer
                                       [&::-webkit-slider-thumb]:hover:bg-blue-300 [&::-webkit-slider-thumb]:transition-colors
                                       [&::-moz-range-thumb]:appearance-none [&::-moz-range-thumb]:w-4 [&::-moz-range-thumb]:h-4
                                       [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:bg-blue-400 [&::-moz-range-thumb]:border-2
                                       [&::-moz-range-thumb]:border-white/20 [&::-moz-range-thumb]:shadow-lg [&::-moz-range-thumb]:cursor-pointer
                                       [&::-moz-range-thumb]:hover:bg-blue-300 [&::-moz-range-thumb]:border-none
                                       focus:outline-none focus:ring-2 focus:ring-blue-400/50 focus:ring-offset-2 focus:ring-offset-transparent"
                                style={`background: linear-gradient(to right, rgb(96 165 250) 0%, rgb(96 165 250) ${currentVolume()}%, rgba(255,255,255,0.1) ${currentVolume()}%, rgba(255,255,255,0.1) 100%)`}
                                onInput={async (e) => {
                                    const volume = parseInt(e.currentTarget.value);
                                    await Api.SetAudioDeviceVolumeAsync(device, volume, false);
                                }}
                            />
                        </div>
                    </Show>
                </div>
            }}</For>
        </div>
    </div>
}

function truncateMiddle(text: string | undefined, maxLength: number = 28): string {
    if (!text) {
        return "";
    }

    if (text.length <= maxLength) return text;

    const start = Math.ceil((maxLength - 3) / 2);
    const end = Math.floor((maxLength - 3) / 2);

    return text.slice(0, start) + '...' + text.slice(text.length - end);
}