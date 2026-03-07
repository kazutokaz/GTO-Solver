import Fastify from 'fastify';
import cors from '@fastify/cors';
import jwt from '@fastify/jwt';
import websocket from '@fastify/websocket';
import { config } from './config';
import { authRoutes } from './routes/auth';
import { solveRoutes } from './routes/solve';
import { userRoutes } from './routes/user';
import { billingRoutes } from './routes/billing';
import { websocketRoutes } from './routes/websocket';
import { authPlugin } from './services/auth';

async function main() {
  const app = Fastify({
    logger: true,
  });

  // Plugins
  await app.register(cors, {
    origin: true,
    credentials: true,
  });

  await app.register(jwt, {
    secret: config.jwt.secret,
  });

  await app.register(websocket);

  // Auth decorator
  authPlugin(app);

  // Routes
  await app.register(authRoutes);
  await app.register(solveRoutes);
  await app.register(userRoutes);
  await app.register(billingRoutes);
  await app.register(websocketRoutes);

  // Health check
  app.get('/api/health', async () => {
    return { status: 'ok', timestamp: new Date().toISOString() };
  });

  // Start server
  try {
    await app.listen({ port: config.port, host: config.host });
    console.log(`Server running on http://${config.host}:${config.port}`);
  } catch (err) {
    app.log.error(err);
    process.exit(1);
  }
}

main();
