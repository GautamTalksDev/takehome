export function stripeClient(env) {
  if (env.STRIPE) {
    return env.STRIPE;
  }
  return {
    async createCheckoutSession(input) {
      const body = new URLSearchParams();
      body.set('mode', input.mode ?? 'subscription');
      body.set('success_url', input.success_url);
      body.set('cancel_url', input.cancel_url);
      body.set('client_reference_id', input.client_reference_id);
      body.set('metadata[plan]', input.metadata.plan);
      body.set('line_items[0][price]', input.price);
      body.set('line_items[0][quantity]', '1');
      const response = await fetch(
        'https://api.stripe.com/v1/checkout/sessions',
        {
          method: 'POST',
          headers: {
            authorization: `Bearer ${env.STRIPE_SECRET_KEY}`,
            'content-type': 'application/x-www-form-urlencoded',
          },
          body,
        },
      );
      const json = await response.json();
      if (!response.ok) {
        throw new Error(json.error?.message ?? 'stripe_checkout_failed');
      }
      return {
        ...json,
        currency: json.currency ?? 'cad',
        price: input.price,
      };
    },
    constructEvent(raw) {
      return JSON.parse(raw);
    },
  };
}
