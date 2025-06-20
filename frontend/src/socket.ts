import { createSignal, createEffect, onCleanup } from 'solid-js';
import {Api} from "./api.ts";

type MessageHandler<T = any> = (data: T) => void;

interface WebSocketStore {
    isConnected: () => boolean;
    subscribe: <T = any>(handler: MessageHandler<T>) => () => void;
    getLastMessage: () => any;
}

function createWebSocketStore(url: string): WebSocketStore {
    const [isConnected, setIsConnected] = createSignal(false);
    const [lastMessage, setLastMessage] = createSignal<any>(null);

    let ws: WebSocket | null = null;
    const subscribers = new Set<MessageHandler>();
    let reconnectTimeout: number | null = null;

    const connect = () => {
        if (ws?.readyState === WebSocket.OPEN) return;

        ws = new WebSocket(url);

        ws.onopen = () => {
            setIsConnected(true);
            console.log('WebSocket connected');
        };

        ws.onmessage = (event) => {
            let data;
            try {
                data = JSON.parse(event.data);
            } catch {
                data = event.data;
            }

            setLastMessage(data);

            subscribers.forEach(handler => {
                try {
                    handler(data);
                } catch (error) {
                    console.error('Error in WebSocket message handler:', error);
                }
            });
        };

        ws.onclose = () => {
            setIsConnected(false);
            console.log('WebSocket disconnected');

            // Auto-reconnect after 3 seconds if there are active subscribers
            if (subscribers.size > 0) {
                reconnectTimeout = window.setTimeout(() => {
                    console.log('Attempting to reconnect...');
                    connect();
                }, 3000);
            }
        };

        ws.onerror = (error) => {
            console.error('WebSocket error:', error);
        };
    };

    const disconnect = () => {
        if (reconnectTimeout) {
            clearTimeout(reconnectTimeout);
            reconnectTimeout = null;
        }

        if (ws) {
            ws.close();
            ws = null;
        }
        setIsConnected(false);
    };

    const subscribe = <T = any>(handler: MessageHandler<T>) => {
        subscribers.add(handler);

        if (subscribers.size === 1) {
            connect();
        }

        return () => {
            subscribers.delete(handler);

            if (subscribers.size === 0) {
                disconnect();
            }
        };
    };

    return {
        isConnected,
        subscribe,
        getLastMessage: lastMessage
    };
}

export const wsStore = createWebSocketStore(Api.GetSockUrl());

export function useWebSocket<T = any>(handler: MessageHandler<T>) {
    let unsubscribe: (() => void) | null = null;

    createEffect(() => {
        unsubscribe = wsStore.subscribe(handler);
    });

    onCleanup(() => {
        unsubscribe?.();
    });

    return {
        isConnected: wsStore.isConnected,
        lastMessage: wsStore.getLastMessage
    };
}