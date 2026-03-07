import { FastifyInstance, FastifyRequest, FastifyReply } from 'fastify';
import { query, queryOne } from '../db/pool';
import { createHash } from 'crypto';

// Simple password hashing (use bcrypt in production)
function hashPassword(password: string): string {
  return createHash('sha256').update(password).digest('hex');
}

function verifyPassword(password: string, hash: string): boolean {
  return hashPassword(password) === hash;
}

export async function signup(email: string, password: string, name?: string) {
  const existing = await queryOne('SELECT id FROM users WHERE email = $1', [email]);
  if (existing) {
    throw new Error('Email already registered');
  }

  const hash = hashPassword(password);
  const rows = await query<{ id: string }>(
    'INSERT INTO users (email, password_hash, name) VALUES ($1, $2, $3) RETURNING id',
    [email, hash, name || null]
  );
  return rows[0];
}

export async function login(email: string, password: string) {
  const user = await queryOne<{ id: string; password_hash: string; plan: string }>(
    'SELECT id, password_hash, plan FROM users WHERE email = $1',
    [email]
  );
  if (!user || !verifyPassword(password, user.password_hash)) {
    throw new Error('Invalid email or password');
  }
  return { id: user.id, plan: user.plan };
}

// JWT auth decorator for protected routes
export function authPlugin(app: FastifyInstance) {
  app.decorate('authenticate', async (request: FastifyRequest, reply: FastifyReply) => {
    try {
      await request.jwtVerify();
    } catch (err) {
      reply.code(401).send({ error: 'Unauthorized' });
    }
  });
}
