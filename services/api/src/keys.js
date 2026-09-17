import { createHash, randomBytes } from 'node:crypto';

export function hashKey(secret) {
  return createHash('sha256').update(secret).digest('hex');
}

export function randomHex(bytes) {
  return randomBytes(bytes).toString('hex');
}

export function mintKey(kind) {
  if (kind !== 'test' && kind !== 'live') {
    throw new Error(`unknown key kind ${kind}`);
  }
  return `np_${kind}_${randomHex(24)}`;
}

export function parseBearer(request) {
  const header = request.headers.get('authorization');
  if (!header) {
    return null;
  }
  const match = header.match(/^Bearer\s+(\S+)$/i);
  return match ? match[1] : null;
}

export function keyKind(secret) {
  if (secret.startsWith('np_test_')) {
    return 'test';
  }
  if (secret.startsWith('np_live_')) {
    return 'live';
  }
  return null;
}

export function issueKeyPair(store, accountId) {
  const testKey = mintKey('test');
  const liveKey = mintKey('live');
  store.insertKey({
    account_id: accountId,
    kind: 'test',
    prefix: 'np_test_',
    hash: hashKey(testKey),
  });
  store.insertKey({
    account_id: accountId,
    kind: 'live',
    prefix: 'np_live_',
    hash: hashKey(liveKey),
  });
  return { testKey, liveKey };
}
