import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const source = await readFile(new URL('../src/services/scanCache.ts', import.meta.url), 'utf8');
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2021 } });
const { createScanCache } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);
const deferred = () => {
  let resolve;
  const promise = new Promise((r) => { resolve = r; });
  return { promise, resolve };
};

test('page remounts and refresh clicks share an active scan', async () => {
  const cache = createScanCache();
  const pending = deferred();
  let calls = 0;
  const load = () => { calls++; return pending.promise; };
  const first = cache.get('disk-A', load);
  assert.equal(cache.get('disk-A', load), first);
  assert.equal(cache.get('disk-A', load, true), first);
  pending.resolve([]);
  await first;
  assert.equal(calls, 1);
  assert.deepEqual(await cache.get('disk-A', load), []);
  assert.equal(calls, 1, 'empty results are cached too');
});

test('manual rescan and different filters fetch fresh results', async () => {
  const cache = createScanCache();
  let calls = 0;
  const load = async () => ++calls;
  assert.equal(await cache.get('A:100', load), 1);
  assert.equal(await cache.get('A:100', load, true), 2);
  assert.equal(await cache.get('A:500', load), 3);
  assert.equal(await cache.get('B:100', load), 4);
});

test('failed scans can be retried and expired scans are refreshed', async () => {
  const cache = createScanCache(0);
  await assert.rejects(cache.get('A', async () => { throw new Error('disk unavailable'); }));
  assert.equal(await cache.get('A', async () => 1), 1);
  assert.equal(await cache.get('A', async () => 2), 2);
});

test('cleanup invalidation prevents older work from replacing fresh data', async () => {
  const cache = createScanCache();
  const old = deferred();
  const first = cache.get('A', () => old.promise);
  cache.clear();
  assert.equal(await cache.get('A', async () => 'fresh'), 'fresh');
  old.resolve('old');
  await first;
  assert.equal(await cache.get('A', async () => 'unexpected'), 'fresh');
});

test('completed cache entries are bounded', async () => {
  const cache = createScanCache(60_000, 2);
  await cache.get('A', async () => 1);
  await cache.get('B', async () => 2);
  await cache.get('C', async () => 3);
  assert.equal(await cache.get('A', async () => 4), 4);
});
