/**
 * Outbound mail. Tests and `npm run local` use env.MAILBOX (in-memory).
 * Production and staging use Resend when RESEND_API_KEY is set.
 * Signup must not claim success when neither sink nor Resend is available.
 */

export const RESEND_URL = 'https://api.resend.com/emails';

export class MailTransportError extends Error {
  constructor(message = 'Outbound mail is not configured.') {
    super(message);
    this.name = 'MailTransportError';
  }
}

function hasMailbox(env) {
  return Array.isArray(env?.MAILBOX) || typeof env?.MAILBOX?.push === 'function';
}

/**
 * Deliver one message. Rejects with MailTransportError when production
 * has no transport. Never throws on CR/LF injection — those are dropped.
 * Optional message.html and message.replyTo map to Resend fields.
 */
export async function sendMail(env, message) {
  const to = String(message.to ?? '');
  const subject = String(message.subject ?? '');
  const text = String(message.text ?? '');
  const html = message.html === undefined ? undefined : String(message.html);
  const replyTo = message.replyTo === undefined ? undefined : String(message.replyTo);
  if (
    /[\r\n\0]/.test(to) ||
    /[\r\n\0]/.test(subject) ||
    (replyTo !== undefined && /[\r\n\0]/.test(replyTo))
  ) {
    return;
  }
  if (hasMailbox(env)) {
    env.MAILBOX.push(message);
    return;
  }
  const apiKey = env?.RESEND_API_KEY;
  if (!apiKey) {
    throw new MailTransportError(
      'Outbound mail is not configured. Set RESEND_API_KEY (and MAIL_FROM).',
    );
  }
  const from = String(env.MAIL_FROM ?? '').trim();
  if (!from) {
    throw new MailTransportError('MAIL_FROM is required when using Resend.');
  }
  const body = {
    from,
    to: [to],
    subject,
    text,
  };
  if (html !== undefined && html !== '') {
    body.html = html;
  }
  const reply = (replyTo ?? String(env.MAIL_REPLY_TO ?? '')).trim();
  if (reply) {
    body.reply_to = reply;
  }
  const fetchFn = typeof env.fetch === 'function' ? env.fetch.bind(env) : fetch;
  const response = await fetchFn(RESEND_URL, {
    method: 'POST',
    headers: {
      authorization: `Bearer ${apiKey}`,
      'content-type': 'application/json',
    },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    const detail = await response.text().catch(() => '');
    throw new MailTransportError(
      `Resend rejected the message (${response.status}). ${detail}`.trim(),
    );
  }
}
