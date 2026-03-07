import { FastifyInstance } from 'fastify';
import { WebSocket } from 'ws';

// Simple in-memory map of userId -> WebSocket connections
const connections = new Map<string, Set<WebSocket>>();

export function addConnection(userId: string, ws: WebSocket) {
  if (!connections.has(userId)) {
    connections.set(userId, new Set());
  }
  connections.get(userId)!.add(ws);

  ws.on('close', () => {
    connections.get(userId)?.delete(ws);
    if (connections.get(userId)?.size === 0) {
      connections.delete(userId);
    }
  });
}

export function notifyUser(userId: string, event: string, data: any) {
  const userConnections = connections.get(userId);
  if (!userConnections) return;

  const message = JSON.stringify({ event, data });
  for (const ws of userConnections) {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(message);
    }
  }
}

export async function websocketRoutes(app: FastifyInstance) {
  app.get('/ws', { websocket: true }, (socket, request) => {
    // Expect token as query param for WebSocket auth
    const url = new URL(request.url, `http://${request.headers.host}`);
    const token = url.searchParams.get('token');

    if (!token) {
      socket.close(4001, 'Missing token');
      return;
    }

    try {
      const decoded = app.jwt.verify(token) as { userId: string };
      addConnection(decoded.userId, socket);

      socket.on('message', (msg: Buffer) => {
        // Handle ping/pong or client messages
        const text = msg.toString();
        if (text === 'ping') {
          socket.send('pong');
        }
      });
    } catch {
      socket.close(4002, 'Invalid token');
    }
  });
}
