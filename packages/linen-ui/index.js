export function createMotion(root, isReady) {
const reducedMotion=matchMedia('(prefers-reduced-motion: reduce)');
const motion={duration:200,easing:'cubic-bezier(0.16, 1, 0.3, 1)'};
const contentValues = new WeakMap();
function setContent(id, value, html = false) {
  const element = typeof id === 'string' ? document.getElementById(id) : id;
  value = String(value);
  if (contentValues.get(element) === value) return;
  const old = element.querySelector(':scope > .copy-current');
  const oldPosition = old ? [old.offsetLeft, old.offsetTop] : [0, 0];
  const oldWidth = element.getBoundingClientRect().width;
  const oldHeight = element.getBoundingClientRect().height;
  element.querySelectorAll(':scope > .copy-outgoing').forEach(node => node.remove());
  const content = document.createElement('span'); content.className = 'copy-current';
  if (html) content.innerHTML = value; else content.textContent = value;
  element.replaceChildren(content);
  contentValues.set(element, value);
  element.classList.add('motion-copy');
  if (old && isReady() && !reducedMotion.matches) {
    if ((['pause','mode','volume'].includes(element.id))) element.animate([{ width: `${oldWidth}px` }, { width: `${element.getBoundingClientRect().width}px` }], motion);
    const height = element.getBoundingClientRect().height;
    if ((element.closest('.message') || element.id === 'profile-hint') && Math.abs(height - oldHeight) > .5) element.animate([{ height: `${oldHeight}px` }, { height: `${height}px` }], motion);
    old.className = 'copy-outgoing'; old.style.left = `${oldPosition[0]}px`; old.style.top = `${oldPosition[1]}px`; old.setAttribute('aria-hidden', 'true'); element.append(old);
    old.animate([{ opacity: 1 }, { opacity: 0 }], { ...motion, fill: 'forwards' }).finished.finally(() => old.remove()).catch(() => {});
    content.animate([{ opacity: 0, transform: 'translateY(3px)' }, { opacity: 1, transform: 'none' }], motion);
  }
}
const setText = (id, value) => setContent(id, value);
const views = root.querySelector('#views');
const native = document.body.classList.contains('native');
let contentHeight = 0;
function animateHeight(before) {
  const active = views.querySelector(':scope > section:not([hidden])');
  contentHeight = active.getBoundingClientRect().height;
  views.getAnimations().forEach(animation => animation.cancel());
  const targetHeight = views.getBoundingClientRect().height;
  if (!native && isReady() && !reducedMotion.matches && Math.abs(targetHeight - before) > .5)
    views.animate([{height:`${before}px`},{height:`${targetHeight}px`}], motion);
}
function animateView(before, changed) {
  if (!native) animateHeight(before);
  if (changed && isReady() && !reducedMotion.matches) {
    const incoming = views.querySelector(':scope > section:not([hidden])');
    incoming.getAnimations().forEach(animation => animation.cancel());
    incoming.animate([{opacity:0,transform:'translateY(6px)'},{opacity:1,transform:'none'}], motion);
  }
}
if (!native) {
  const observer = new ResizeObserver(() => requestAnimationFrame(() => {
    const active = views.querySelector(':scope > section:not([hidden])');
    if (Math.abs(active.getBoundingClientRect().height - contentHeight) > .5)
      animateHeight(views.getBoundingClientRect().height);
  }));
  for (const section of views.children) observer.observe(section);
}
return {setContent,setText,animateView,reducedMotion,motion};
}

export function fitPanel(root, invoke) {
  const panel = root.querySelector('.flyout');
  const views = root.querySelector('#views');
  const reduced = matchMedia('(prefers-reduced-motion: reduce)');
  let current = 0, target = 0, from = 0, started = 0, limit = Infinity, reserved = 0, pending = null;
  async function resize() {
    let suspended = false;
    while (true) {
      if (target > reserved || (current !== target && !suspended)) {
        const reservation = Math.max(current || from, target);
        const layout = await invoke('resize_flyout', {height:current || from, reserve:reservation});
        panel.dataset.backdrop = String(layout.backdrop === true);
        suspended = true;
        limit = Math.floor(layout.limit);
        reserved = Math.min(reservation, limit);
        panel.style.maxHeight = limit + 'px';
        if (target > limit) {from = Math.min(current || from, limit); target = limit; started = performance.now();}
        continue;
      }
      const progress = document.body.classList.contains('presented') && !reduced.matches
        ? Math.min(1, (performance.now() - started) / 200) : 1;
      const height = Math.ceil(from + (target - from) * (1 - (1 - progress) ** 3));
      panel.style.height = height + 'px';
      const layout = await invoke('resize_flyout', {height});
      panel.dataset.backdrop = String(layout.backdrop === true);
      current = height;
      limit = Math.floor(layout.limit);
      if (target > limit) {from = Math.min(current, limit); target = limit; started = performance.now();}
      panel.style.maxHeight = `${limit}px`;
      if (current === target) {
        if (document.body.classList.contains('presented')) {
          await new Promise(requestAnimationFrame);
          await new Promise(requestAnimationFrame);
        }
        if (current !== target) continue;
        const settled = await invoke('resize_flyout', {height:current, settled:true});
        panel.dataset.backdrop = String(settled.backdrop === true);
        reserved = current;
        if (current === target) return;
      }
      if (document.body.classList.contains('presented')) await new Promise(requestAnimationFrame);
    }
  }
  function fit(refresh = false) {
    const max = panel.style.maxHeight;
    panel.style.maxHeight = 'none';
    const previousHeight = panel.style.height;
    panel.style.height = 'auto';
    const natural = Math.ceil(panel.scrollHeight);
    panel.style.height = previousHeight;
    panel.style.maxHeight = max;
    views.classList.toggle('scroll', natural > limit);
    const height = Math.min(natural, limit);
    if (height !== target) {
      from = panel.getBoundingClientRect().height || current || height;
      target = height;
      started = performance.now();
    }
    if (!pending && (refresh === true || current !== target)) pending = resize().finally(() => {pending = null;});
    return pending || Promise.resolve();
  }
  const observer = new ResizeObserver(() => requestAnimationFrame(fit));
  for (const element of [panel.querySelector('.current'), ...views.children, panel.querySelector('#error-wrap')]) observer.observe(element);
  return fit;
}
export function flyout({name,card,main,settings,gear}) {
    return `<section class="flyout" aria-label="${name}">${card}<div id="views">${main}<section id="settings" hidden><div class="settings">${settings}</div></section></div><div id="error-wrap" class="message" aria-hidden="true"><div><p id="error" role="alert"></p></div></div><div class="footer-area"><div class="divider"></div><footer><button id="pause" class="button" type="button"></button><button id="gear" class="button icon-button" aria-expanded="false" aria-controls="settings">${gear}</button></footer></div></section>`;
}
export const statusCard=(extra='')=>`<div class="current"><span class="badge" id="current-icon"></span><div class="current-copy"><div id="current-name"></div><p id="current-status" role="status"></p></div>${extra}</div>`;
export const languageSettings=(name,version,open)=>`<h2 id="language-label"></h2><div class="segments" role="group" aria-labelledby="language-label"><button data-language="system"></button><button data-language="zh">简体中文</button><button data-language="en">English</button></div><div class="about"><span>${name} ${version}</span><a id="github" href="https://github.com/Hezric/${name}" target="_blank" rel="noreferrer">GitHub ${open}</a></div>`;
export const pauseSettings=()=>`<h2 id="pause-for-label"></h2><div class="pause-row"><div class="segments presets" role="group" aria-labelledby="pause-for-label"><button data-minutes="15">15</button><button data-minutes="30">30</button><button data-minutes="60">60</button><button data-minutes="120">120</button></div><input id="pause-minutes" class="pause-minutes text-input" type="text" inputmode="numeric" maxlength="4" aria-labelledby="pause-for-label" aria-describedby="pause-error" autocomplete="off" spellcheck="false"><span id="minutes-unit" class="minutes-unit"></span></div><div id="pause-error-wrap" class="message" aria-hidden="true"><div><p id="pause-error" class="pause-error" role="alert"></p></div></div>`;
