import { useEffect, useRef, useCallback } from 'react';
import { useAuthStore } from '../store/authStore';

type MessageHandler = (event: string, data: any) => void;

export function useWebSocket(onMessage?: MessageHandler) {
  const { token, isAuthenticated } = useAuthStore();
  const wsRef = useRef<WebSocket | null>(null);
  const handlersRef = useRef<Set<MessageHandler>>(new Set());

  const addHandler = useCallback((handler: MessageHandler) => {
    handlersRef.current.add(handler);
    return () => { handlersRef.current.delete(handler); };
  }, []);

  useEffect(() => {
    if (onMessage) {
      handlersRef.current.add(onMessage);
      return () => { handlersRef.current.delete(onMessage); };
    }
  }, [onMessage]);

  useEffect(() => {
    if (!isAuthenticated || !token) return;

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/ws?token=${token}`;

    let ws: WebSocket;
    let reconnectTimer: ReturnType<typeof setTimeout>;
    let pingTimer: ReturnType<typeof setInterval>;

    const connect = () => {
      ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        // Keep alive with ping every 30s
        pingTimer = setInterval(() => {
          if (ws.readyState === WebSocket.OPEN) {
            ws.send('ping');
          }
        }, 30000);
      };

      ws.onmessage = (event) => {
        if (event.data === 'pong') return;
        try {
          const { event: evt, data } = JSON.parse(event.data);
          for (const handler of handlersRef.current) {
            handler(evt, data);
          }
        } catch {
          // ignore parse errors
        }
      };

      ws.onclose = (event) => {
        clearInterval(pingTimer);
        if (event.code !== 4001 && event.code !== 4002) {
          // Reconnect after 3s unless auth failure
          reconnectTimer = setTimeout(connect, 3000);
        }
      };

      ws.onerror = () => {
        ws.close();
      };
    };

    connect();

    return () => {
      clearInterval(pingTimer);
      clearTimeout(reconnectTimer);
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
    };
  }, [isAuthenticated, token]);

  return { addHandler };
}
