import test from 'node:test';
import assert from 'node:assert/strict';
import {flyout,statusCard,pauseSettings,languageSettings} from './index.js';

test('both consumers get one accessible shell with independent main and settings views',()=>{
  for(const name of ['Earmuff','Homing']) {
    const html=flyout({name,icon:'icon',card:statusCard(),main:'<section id="main">content</section>',settings:pauseSettings()+languageSettings(name,'0.1.0','open'),gear:'gear'});
    assert.match(html,new RegExp(`aria-label="${name}"`));
    for(const id of ['current-status','views','settings','pause','gear','pause-error','language-label','github'])assert.equal(html.split(`id="${id}"`).length-1,1,id);
    assert.match(html,/aria-controls="settings"/);
    assert.match(html,/aria-describedby="pause-error"/);
    assert.match(html,/role="status"/);
    assert.match(html,/data-language="zh"/);
  }
});
