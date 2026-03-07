import { FastifyInstance, FastifyRequest } from 'fastify';
import Stripe from 'stripe';
import { config } from '../config';
import { createCheckoutSession, handleStripeWebhook } from '../services/billing';
import { queryOne } from '../db/pool';

export async function billingRoutes(app: FastifyInstance) {
  // Subscribe to a plan
  app.post('/api/billing/subscribe', {
    preHandler: [(app as any).authenticate],
  }, async (request: FastifyRequest, reply) => {
    const userId = (request.user as any).userId;
    const { plan } = request.body as { plan: string };

    if (!plan || !config.plans[plan]) {
      return reply.code(400).send({ error: 'Invalid plan' });
    }

    try {
      const url = await createCheckoutSession(userId, plan);
      return { url };
    } catch (err: any) {
      return reply.code(400).send({ error: err.message });
    }
  });

  // Get billing status
  app.get('/api/billing/status', {
    preHandler: [(app as any).authenticate],
  }, async (request: FastifyRequest) => {
    const userId = (request.user as any).userId;
    const user = await queryOne<{
      plan: string; stripe_customer_id: string; billing_cycle_start: string;
    }>(
      'SELECT plan, stripe_customer_id, billing_cycle_start FROM users WHERE id = $1',
      [userId]
    );
    return {
      plan: user?.plan || 'free',
      billingCycleStart: user?.billing_cycle_start,
    };
  });

  // Stripe webhook
  app.post('/api/billing/webhook', {
    config: { rawBody: true },
  }, async (request, reply) => {
    const sig = request.headers['stripe-signature'] as string;
    if (!sig || !config.stripe.webhookSecret || !config.stripe.secretKey) {
      return reply.code(400).send({ error: 'Stripe not configured' });
    }

    try {
      const stripe = new Stripe(config.stripe.secretKey);
      const event = stripe.webhooks.constructEvent(
        (request as any).rawBody || JSON.stringify(request.body),
        sig,
        config.stripe.webhookSecret
      );
      await handleStripeWebhook(event);
      return { received: true };
    } catch (err: any) {
      return reply.code(400).send({ error: err.message });
    }
  });
}
