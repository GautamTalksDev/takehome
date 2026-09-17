import { D1Store } from './store-d1.js';

export { MemoryStore } from './store-memory.js';

export function getStore(env) {
  if (env.STORE) {
    return env.STORE;
  }
  if (env.DB) {
    return new D1Store(env.DB);
  }
  return null;
}
