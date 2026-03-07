import { FastifyInstance } from 'fastify';
import { signup, login } from '../services/auth';
import { z } from 'zod';

const signupSchema = z.object({
  email: z.string().email(),
  password: z.string().min(8),
  name: z.string().optional(),
});

const loginSchema = z.object({
  email: z.string().email(),
  password: z.string(),
});

export async function authRoutes(app: FastifyInstance) {
  app.post('/api/auth/signup', async (request, reply) => {
    const body = signupSchema.parse(request.body);
    try {
      const user = await signup(body.email, body.password, body.name);
      const token = app.jwt.sign({ userId: user.id });
      return { token, userId: user.id };
    } catch (err: any) {
      reply.code(400).send({ error: err.message });
    }
  });

  app.post('/api/auth/login', async (request, reply) => {
    const body = loginSchema.parse(request.body);
    try {
      const user = await login(body.email, body.password);
      const token = app.jwt.sign({ userId: user.id, plan: user.plan });
      return { token, userId: user.id, plan: user.plan };
    } catch (err: any) {
      reply.code(401).send({ error: err.message });
    }
  });
}
