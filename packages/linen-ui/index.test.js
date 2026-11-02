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
  const panel={style:{},dataset:{},scrollHeight:natural,querySelector:()=>({}),getBoundingClientRect(){return {height:parseFloat(this.style.height)||natural};}};
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
test('an unchanged open panel refreshes geometry and recovers after the work area grows',async t=>{
  const root=panelEnvironment(t,300);
  let limit=800, calls=0, applied;
  const fit=fitPanel(root,async(_,args)=>{calls++;applied=args;return {limit};});
  await fit();
  const initial=calls;
  limit=200;
  await fit(true);
  assert.ok(calls>initial,'unchanged content must still re-read native geometry');
  assert.equal(applied.height,200);
  limit=800;
  await fit(true);
  await fit(true);
  assert.equal(applied.height,300,'larger work areas must restore the natural height');
});
test('shrinking keeps the live material inside the DOM until its smaller clip is acknowledged', async t=>{
  const root=panelEnvironment(t,300), panel=root.querySelector('.flyout');
  const entered=Promise.withResolvers(), release=Promise.withResolvers();
  let shrinking=false;
  const fit=fitPanel(root,async(_,args)=>{
    if(shrinking && args.height===180 && !args.settled) {
      assert.equal(panel.style.height,'300px','the old card must contain the material until native clip commit');
      assert.equal(panel.dataset.backdrop,'true','material stays active during a transition');
      entered.resolve();
      await release.promise;
    }
    return {limit:800,backdrop:true};
  });
  await fit();
  shrinking=true;
  panel.scrollHeight=180;
  const pending=fit();
  const state=await Promise.race([entered.promise.then(()=> 'waiting'),pending.then(()=> 'finished')]);
  assert.equal(state,'waiting');
  assert.equal(panel.style.height,'300px');
  release.resolve();
  await pending;
  assert.equal(panel.style.height,'180px');
  assert.equal(panel.dataset.backdrop,'true');
});

test('growth reveals the card before extending its live material', async t=>{
  const root=panelEnvironment(t,180), panel=root.querySelector('.flyout');
  let growing=false;
  const fit=fitPanel(root,async(_,args)=>{
    if(growing && args.height===300 && !args.settled)
      assert.equal(panel.style.height,'300px','card growth contains the next native brush');
    return {limit:800,backdrop:true};
  });
  await fit();
  growing=true;
  panel.scrollHeight=300;
  await fit();
  assert.equal(panel.style.height,'300px');
  assert.equal(panel.dataset.backdrop,'true');
});

test('a target changed during settlement stays contained without losing material', async t=>{
  const root=panelEnvironment(t,300), panel=root.querySelector('.flyout');
  const settling=Promise.withResolvers(), resume=Promise.withResolvers(), calls=[];
  let held=false;
  const fit=fitPanel(root,async(_,args)=>{
    calls.push({args,domHeight:panel.style.height});
    if(args.settled && !held) {held=true;settling.resolve();await resume.promise;}
    if(args.height===180 && !args.settled)
      assert.equal(panel.style.height,'300px','retargeted shrink waits for the smaller clip');
    return {limit:800,backdrop:true};
  });
  const pending=fit();
  await settling.promise;
  panel.scrollHeight=180;
  assert.equal(fit(),pending,'a new target shares the pending resize loop');
  resume.resolve();
  await pending;
  assert.deepEqual(calls.filter(({args})=>args.settled).map(({args})=>args.height),[300,180]);
  assert.equal(panel.style.height,'180px');
  assert.equal(panel.dataset.backdrop,'true');
});