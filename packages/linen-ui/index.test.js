import test from 'node:test';
import assert from 'node:assert/strict';
import {flyout,statusCard,pauseSettings,languageSettings,fitPanel} from './index.js';

test('both consumers get one accessible shell with independent main and settings views',()=>{
  for(const name of ['Earmuff','Homing']) {
    const html=flyout({name,icon:'icon',card:statusCard(),main:'<section id="main">content</section>',settings:pauseSettings()+languageSettings(name,'0.1.0','open'),gear:'gear'});
    assert.match(html,new RegExp(`aria-label="${name}"`));
    for(const id of ['current-status','views','settings','pause','gear','pause-error','language-label','github'])assert.equal(html.split(`id="${id}"`).length-1,1,id);
    assert.match(html, /^<section class="flyout"[^>]*><div class="current">/);
    assert.doesNotMatch(html, /class="header"|class="brand-icon"/);
    assert.match(html,/aria-controls="settings"/);
    assert.match(html,/aria-describedby="pause-error"/);
    assert.match(html,/role="status"/);
    assert.match(html,/data-language="zh"/);
  }
});

function panelEnvironment(t, natural) {
  const names=['document','matchMedia','ResizeObserver','requestAnimationFrame'];
  const previous=names.map(name=>[name,Object.getOwnPropertyDescriptor(globalThis,name)]);
  t.after(()=>{for(const [name,descriptor] of previous){if(descriptor)Object.defineProperty(globalThis,name,descriptor);else delete globalThis[name];}});
  globalThis.document={body:{classList:{contains:()=>false}}};
  globalThis.matchMedia=()=>({matches:true});
  globalThis.ResizeObserver=class {observe(){}};
  globalThis.requestAnimationFrame=callback=>queueMicrotask(callback);
  const views={children:[],classList:{toggle(){}}};
  const panel={style:{},scrollHeight:natural,querySelector:()=>({}),getBoundingClientRect(){return {height:parseFloat(this.style.height)||natural};}};
  return {querySelector:selector=>selector==='.flyout'?panel:views};
}

test('window fitting converges under a fractional work-area limit',async t=>{
  const root=panelEnvironment(t,900), heights=[];
  const fit=fitPanel(root,async(_,args)=>{
    heights.push(args.height);
    if(heights.length>4)throw Error('resize did not converge');
    return {limit:800.8};
  });
  await fit();
  await fit();
  assert.ok(heights.length<=3);
  assert.ok(heights.at(-1)<=800);
});

test('a later fit can recover after one rejected native resize',async t=>{
  const root=panelEnvironment(t,300);
  let attempts=0, applied;
  const fit=fitPanel(root,async(_, args)=>{
    if(++attempts===1)throw Error('temporarily unavailable');
    applied=args; return {limit:800};
  });
  await assert.rejects(fit(),/temporarily unavailable/);
  await fit();
  assert.equal(applied.height,300);
  assert.equal(applied.settled,true);
});
test('hidden-window fitting completes before display frames are available',async t=>{
  const root=panelEnvironment(t,300);
  globalThis.requestAnimationFrame=()=>{throw Error('Hidden webviews may suspend frame callbacks');};
  let applied;
  await fitPanel(root,async(_,args)=>{applied=args;return {limit:800};})();
  assert.equal(applied.height,300);
  assert.equal(applied.settled,true);
});