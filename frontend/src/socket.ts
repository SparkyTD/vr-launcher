// websocket-store.ts
import { createSignal, createEffect, onCleanup } from 'solid-js';
import {Api} from "./api.ts";

type MessageHandler<T = any> = (data: T) => void;

interface WebSocketStore {
    isConnected: () => boolean;
    subscribe: <T = any>(handler: MessageHandler<T>) => () => void;
    getLastMessage: () => any;
}

// Create a singleton WebSocket store
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

            // Notify all subscribers
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

        // Connect when first subscriber is added
        if (subscribers.size === 1) {
            connect();
        }

        // Return unsubscribe function
        return () => {
            subscribers.delete(handler);

            // Disconnect when no more subscribers
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

// Create the singleton instance
export const wsStore = createWebSocketStore(Api.StateSock);

// Helper hook for components
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