import {Portal} from "solid-js/web";
import {Show} from "solid-js";

export type ErrorModalContents = {
    title: string,
    text: string,
}

export type ErrorModalProps = {
    contents: ErrorModalContents | null,
    onClose: () => void,
};

export default function Modal(props: ErrorModalProps) {
    return (
        <Portal>
            <Show when={!!props.contents}>
                <div class="fixed inset-0 z-50 flex items-center justify-center">
                    {/* Backdrop */}
                    <div
                        class="fixed inset-0 bg-black/50 backdrop-blur-sm"
                        onClick={props.onClose}
                    />

                    {/* Modal Content */}
                    <div class="relative w-full max-w-md transform transition-all duration-300 ease-out center">
                        <div class="relative bg-white/5 backdrop-blur-xl rounded-2xl border border-white/10 shadow-2xl overflow-hidden">
                            <div class="absolute top-0 left-0 right-0 h-px bg-gradient-to-r from-transparent via-white/40 to-transparent"/>

                            {/* Header */}
                            <div class="text-center p-6 mt-4 border-white/10">
                                <h2 class="text-xl font-semibold text-white">{props.contents?.title}</h2>
                            </div>

                            {/* Content */}
                            <div class="p-6 space-y-4 text-center">
                                <p class="text-white/80 leading-relaxed">
                                    {props.contents?.text}
                                </p>
                            </div>

                            {/* Footer */}
                            <div class="p-6 border-white/10 flex gap-3 justify-end">
                                <button
                                    onClick={props.onClose}
                                    class="px-8 py-4 w-full bg-white/15 hover:bg-white/20 backdrop-blur-sm border border-white/10 rounded-3xl text-white font-medium shadow-lg">
                                    Close
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            </Show>
        </Portal>
    )
}