import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/services/duplicateSelection.ts', import.meta.url), 'utf8');
const { outputText } = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2021 } });
const { keepersFor, duplicateCleanup } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);
const file = (path, modified = '2026-01-01') => ({ path, modified, sizeBytes: 10 });
const a = { id: 'a', kind: 'exact', files: [file('/a'), file('/b', '2026-02-01')] };
const b = { id: 'b', kind: 'exact', files: [file('/c'), file('/d', '2026-02-01')] };
const paths = entries => entries.map(f => f.path).sort();
test('choosing a keeper in one group never deletes all files from another', () => {
 assert.deepEqual(paths(duplicateCleanup([a,b], [a,b], { a: ['/a'] })), ['/b','/c']);
 assert.deepEqual([...keepersFor(b, { a: ['/a'] })], ['/d']);
});
test('similar groups keep every file until explicitly reviewed', () => {
 const similar = { ...a, kind: 'similar' };
 assert.deepEqual(duplicateCleanup([similar], [similar], {}), []);
 assert.deepEqual(paths(duplicateCleanup([similar], [similar], { a: ['/a'] })), ['/b']);
});
test('all retained, empty, or stale keeper choices remain safe', () => {
 assert.deepEqual(duplicateCleanup([a], [a], { a: ['/a','/b'] }), []);
 assert.deepEqual(paths(duplicateCleanup([a], [a], { a: ['/missing'] })), ['/a']);
 assert.deepEqual(paths(duplicateCleanup([a], [a], { a: [] })), ['/a']);
});
test('retained files in overlapping or hidden groups cannot be deleted', () => {
 const overlap = { id: 'overlap', kind: 'exact', files: [file('/a', '2026-12-01'), file('/e')] };
 assert.deepEqual(duplicateCleanup([a,overlap], [a], {}), []);
 assert.deepEqual(paths(duplicateCleanup([a,overlap], [a,overlap], {})), ['/e']);
});
test('only shown groups produce candidates, and paths are unique', () => {
 assert.deepEqual(paths(duplicateCleanup([a,b], [a], {})), ['/a']);
 const duplicate = { ...a, id: 'same' };
 assert.deepEqual(paths(duplicateCleanup([a,duplicate], [a,duplicate], {})), ['/a']);
});

test('protected dependencies stay retained even when another copy is chosen', () => {
 const group={id:'protected',kind:'exact',files:[{...file('/sdk/a.dll'),protectedReason:'应用依赖'},file('/copy/a.dll')]};
 assert.deepEqual([...keepersFor(group,{protected:['/copy/a.dll']})].sort(),['/copy/a.dll','/sdk/a.dll']);
 assert.deepEqual(duplicateCleanup([group],[group],{protected:['/copy/a.dll']}),[]);
});
