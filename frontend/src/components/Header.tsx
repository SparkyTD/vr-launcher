import {Gamepad2Icon} from "lucide-solid";
import AudioSelector from "./AudioSelector.tsx";
import BatteryIndicator from "./BatteryIndicator.tsx";

export default function Header() {
    return <div class="flex justify-between items-center mb-8 z-50 relative">
        <div class="flex items-center">
            <Gamepad2Icon class="w-8 h-8 mr-3 text-indigo-400"/>
            <h1 class="text-2xl font-medium tracking-wider">Sparky's VR Launcher</h1>
        </div>

        <div class="flex items-center space-x-4">
            <AudioSelector/>
            <BatteryIndicator/>
        </div>
    </div>
}