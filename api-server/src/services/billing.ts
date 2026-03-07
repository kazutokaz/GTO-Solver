import Stripe from 'stripe';
import { config } from '../config';
import { query, queryOne } from '../db/pool';

const stripe = config.stripe.secretKey
  ? new Stripe(config.stripe.secretKey)
  : null;

export async function checkSolveLimit(userId: string): Promise<boolean> {
  const user = await queryOne<{ plan: string; solve_limit: number; solves_used_this_month: number }>(
    'SELECT plan, solve_limit, solves_used_this_month FROM users WHERE id = $1',
    [userId]
  );
  if (!user) return false;
  if (user.solve_limit === -1) return true; // unlimited
  return user.solves_used_this_month < user.solve_limit;
}

export async function incrementSolveCount(userId: string): Promise<void> {
  await query(
    'UPDATE users SET solves_used_this_month = solves_used_this_month + 1, updated_at = NOW() WHERE id = $1',
    [userId]
  );
}

export async function createCheckoutSession(userId: string, plan: string): Promise<string | null> {
  if (!stripe) throw new Error('Stripe not configured');

  const planConfig = config.plans[plan];
  if (!planConfig || !planConfig.priceId) throw new Error('Invalid plan');

  const user = await queryOne<{ email: string; stripe_customer_id: string | null }>(
    'SELECT email, stripe_customer_id FROM users WHERE id = $1',
    [userId]
  );
  if (!user) throw new Error('User not found');

  let customerId = user.stripe_customer_id;
  if (!customerId) {
    const customer = await stripe.customers.create({ email: user.email });
    customerId = customer.id;
    await query('UPDATE users SET stripe_customer_id = $1 WHERE id = $2', [customerId, userId]);
  }

  const session = await stripe.checkout.sessions.create({
    customer: customerId,
    mode: 'subscription',
    line_items: [{ price: planConfig.priceId, quantity: 1 }],
    success_url: `${process.env.FRONTEND_URL || 'http://localhost:5173'}/app/settings?success=true`,
    cancel_url: `${process.env.FRONTEND_URL || 'http://localhost:5173'}/app/settings?canceled=true`,
    metadata: { userId, plan },
  });

  return session.url;
}

export async function handleStripeWebhook(event: Stripe.Event): Promise<void> {
  switch (event.type) {
    case 'invoice.paid': {
      const invoice = event.data.object as Stripe.Invoice;
      const customerId = invoice.customer as string;
      const user = await queryOne<{ id: string }>(
        'SELECT id FROM users WHERE stripe_customer_id = $1',
        [customerId]
      );
      if (user) {
        // Reset monthly usage on successful payment
        await query(
          'UPDATE users SET solves_used_this_month = 0, billing_cycle_start = NOW(), updated_at = NOW() WHERE id = $1',
          [user.id]
        );
      }
      break;
    }
    case 'customer.subscription.deleted': {
      const sub = event.data.object as Stripe.Subscription;
      const customerId = sub.customer as string;
      await query(
        "UPDATE users SET plan = 'free', solve_limit = 10, updated_at = NOW() WHERE stripe_customer_id = $1",
        [customerId]
      );
      break;
    }
    case 'customer.subscription.updated': {
      const sub = event.data.object as Stripe.Subscription;
      const customerId = sub.customer as string;
      const priceId = sub.items.data[0]?.price.id;
      // Find matching plan
      for (const [planName, planCfg] of Object.entries(config.plans)) {
        if (planCfg.priceId === priceId) {
          await query(
            'UPDATE users SET plan = $1, solve_limit = $2, updated_at = NOW() WHERE stripe_customer_id = $3',
            [planName, planCfg.solveLimit, customerId]
          );
          break;
        }
      }
      break;
    }
  }
}
