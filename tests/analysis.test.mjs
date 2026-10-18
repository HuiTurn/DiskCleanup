import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const moduleUrl = (s) => `data:text/javascript;base64,${Buffer.from(s).toString('base64')}`;
const mockUrl = moduleUrl(`
export const state = { starts: 0, cancels: 0, cleans: 0, failClean: false, fail: false, handlers: {} };
let ready;
const gate = new Promise(r => ready = r);
export const release = () => ready();
export const bridge = {
 getPlatform: async () => 'macos', getDisks: async () => [{isSystem:true,mountPoint:'/'}], getSettings: async () => ({}),
 startScan: async () => {state.starts++; if(state.fail) throw new Error('permission denied');},
 cleanJunk: async () => {state.cleans++; if(state.failClean) throw new Error('file busy');},
 getJunkCategories: async () => [], lastCleanupRecord: async () => ({items:[]}),
 cancelScan: async () => {state.cancels++;}
};
export const onScanProgress = async (cb) => { await gate; state.handlers.progress = cb; return () => {}; };
export const onScanFinished = async (cb) => { state.handlers.finished = cb; return () => {}; };
export const onCleanFinished = async (cb) => { state.handlers.cleaned = cb; return () => {}; };
`);
const mock = await import(mockUrl);
// Node 环境没有 window，等价于非 mock；只需满足 appStore 的命名导入。
const mockModeUrl = moduleUrl('export const isMockMode = false; export const mockState = {};');
const source = await readFile(new URL('../src/stores/appStore.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, {compilerOptions: {module:ts.ModuleKind.ESNext,target:ts.ScriptTarget.ES2021}}).outputText
 .replace('"zustand"', JSON.stringify(import.meta.resolve('zustand')))
 .replace('"../services/bridge"', JSON.stringify(mockUrl))
 .replace('"../services/mock"', JSON.stringify(mockModeUrl));
const { useApp } = await import(moduleUrl(compiled));
const until = async (predicate) => { for(let i=0;i<100;i++){if(predicate())return;await new Promise(setImmediate);} assert.fail('state did not settle'); };
const result = {totalBytes:1000,totalFiles:2,root:'/',categories:[],topFolders:[],dirNames:[],sizeBuckets:[]};
const idle = () => useApp.setState({page:'home',scanResult:null,scanStatus:{running:false,percent:0,scannedBytes:0,scannedFiles:0,currentPath:'',recentFiles:[]}});

test('sidebar starts a real command only after listeners are ready; repeated clicks do not duplicate it', async () => {
 useApp.getState().go('analyze');
 useApp.getState().go('analyze');
 await new Promise(setImmediate);
 assert.equal(useApp.getState().page,'analyze');
 assert.equal(mock.state.starts,0);
 mock.release();
 await until(() => !useApp.getState().scanStarting);
 assert.equal(mock.state.starts,1);
 assert.equal(useApp.getState().scanStatus.currentPath,'/');
});

test('completion waits for the result and re-entry shows results without rescanning', () => {
 mock.state.handlers.progress({...useApp.getState().scanStatus,running:false});
 assert.equal(useApp.getState().page,'analyze');
 mock.state.handlers.finished(result);
 assert.equal(useApp.getState().page,'results');
 useApp.getState().go('home');
 useApp.getState().go('analyze');
 assert.equal(useApp.getState().page,'results');
 assert.equal(mock.state.starts,1);
});

test('a scan finishing in the background does not pull the user away from another page', async () => {
 idle(); await useApp.getState().startScan();
 useApp.getState().go('settings');
 mock.state.handlers.finished(result);
 assert.equal(useApp.getState().page,'settings');
 assert.equal(useApp.getState().scanResult,result);
});

test('cancel stops waiting and ignores late completion events', async () => {
 idle(); await useApp.getState().startScan();
 await useApp.getState().cancelScan();
 mock.state.handlers.finished(result);
 assert.equal(useApp.getState().page,'home');
 assert.equal(useApp.getState().scanResult,null);
 assert.equal(useApp.getState().scanStatus.running,false);
});

test('start failure leaves a visible retryable error instead of an endless 0% spinner', async () => {
 idle(); mock.state.fail=true;
 await useApp.getState().startScan();
 assert.equal(useApp.getState().scanStatus.running,false);
 assert.match(useApp.getState().scanError,/permission denied/);
 mock.state.fail=false;
 await useApp.getState().startScan();
 assert.equal(useApp.getState().scanError,null);
 assert.equal(useApp.getState().scanStatus.running,true);
});

test('filesystem errors delivered by the worker stop the scan', () => {
 mock.state.handlers.progress({...useApp.getState().scanStatus,running:false,error:'无法读取目录'});
 assert.equal(useApp.getState().scanError,'无法读取目录');
 assert.equal(useApp.getState().scanStatus.running,false);
});


test('cleanup stays busy until completion and blocks duplicate submissions across pages', async () => {
 await useApp.getState().cleanSelected(['/a','/b']);
 assert.equal(mock.state.cleans,0,'selection must not delete before review');
 await useApp.getState().confirmCleanup();
 useApp.getState().go('large-files');
 await useApp.getState().cleanSelected(['/c']);
 assert.equal(mock.state.cleans,1);
 assert.equal(useApp.getState().cleaning,true);
 assert.equal(useApp.getState().cleanRequested,2);
 mock.state.handlers.cleaned({files:1,freedBytes:10,durationMs:5});
 assert.equal(useApp.getState().cleaning,false);
 assert.equal(useApp.getState().page,'done');
 assert.equal(useApp.getState().cleanSummary.files,1);
 assert.equal(useApp.getState().cleanRequested,2,'partial completion remains identifiable');
});

test('cleanup command failure releases busy state and displays the error', async () => {
 mock.state.failClean=true;
 await useApp.getState().cleanSelected(['/a']);
 await useApp.getState().confirmCleanup();
 assert.equal(useApp.getState().cleaning,false);
 assert.match(useApp.getState().cleanError,/file busy/);
 mock.state.failClean=false;
 await useApp.getState().cleanSelected(['/a']);
 await useApp.getState().confirmCleanup();
 assert.equal(useApp.getState().cleanError,null);
 assert.equal(useApp.getState().cleaning,true);
 mock.state.handlers.cleaned({files:1,freedBytes:10,durationMs:5});
});

test('cancelling cleanup review never invokes the deletion command', async () => {
 const before=mock.state.cleans;
 await useApp.getState().cleanSelected(['/sensitive-file']);
 assert.deepEqual(useApp.getState().pendingCleanup,['/sensitive-file']);
 useApp.getState().cancelCleanup();
 await useApp.getState().confirmCleanup();
 assert.equal(mock.state.cleans,before);
});
