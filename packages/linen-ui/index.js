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
function animateView(before, outgoing) {
  const views = document.querySelector('#views');
  views.getAnimations().forEach(animation => animation.cancel());
  views.querySelectorAll(':scope > .view-outgoing').forEach(node => node.remove());
  const height = views.getBoundingClientRect().height;
  if (!isReady() || reducedMotion.matches) return;
  if (Math.abs(height - before) > .5) views.animate([{ height: `${before}px` }, { height: `${height}px` }], motion);
  if (outgoing) {
    outgoing.removeAttribute('id'); outgoing.querySelectorAll('[id]').forEach(node => node.removeAttribute('id'));
    outgoing.classList.add('view-outgoing'); outgoing.inert = true; outgoing.setAttribute('aria-hidden', 'true');
    outgoing.style.height = `${before}px`; views.append(outgoing);
    outgoing.animate([{ opacity: 1 }, { opacity: 0 }], { ...motion, fill: 'forwards' }).finished.finally(() => outgoing.remove()).catch(() => {});
    views.querySelector(':scope > section:not([hidden])').animate([{ opacity: 0, transform: 'translateY(6px)' }, { opacity: 1, transform: 'none' }], motion);
  }
}

return {setContent,setText,animateView,reducedMotion,motion};
}

export function fitPanel(root, invoke) {
    let lastHeight=0, pending=Promise.resolve();
    return function fit() {
        const panel=root.querySelector('.flyout');
        const max=panel.style.maxHeight;
        panel.style.maxHeight='none';
        const height=Math.ceil(panel.scrollHeight);
        panel.style.maxHeight=max;
        if(height===lastHeight) return pending;
        lastHeight=height;
        pending=pending.catch(()=>{}).then(async()=>{
            const layout=await invoke('resize_flyout',{height:lastHeight});
            panel.style.maxHeight=`${layout.limit}px`;
            root.querySelector('#views').classList.toggle('scroll',lastHeight>layout.limit);
        });
        return pending;
    };
}

export function flyout({name,icon,card,main,settings,gear}) {
    return `<section class="flyout" aria-label="${name}"><header class="header"><span class="brand-icon">${icon}</span><span class="brand">${name}</span></header>${card}<div id="views">${main}<section id="settings" hidden><div class="settings">${settings}</div></section></div><div id="error-wrap" class="message" aria-hidden="true"><div><p id="error" role="alert"></p></div></div><div class="footer-area"><div class="divider"></div><footer><button id="pause" class="button" type="button"></button><button id="gear" class="button icon-button" aria-expanded="false" aria-controls="settings">${gear}</button></footer></div></section>`;
}
export const statusCard=(extra='')=>`<div class="current"><span class="badge" id="current-icon"></span><div class="current-copy"><div id="current-name"></div><p id="current-status" role="status"></p></div>${extra}</div>`;
export const languageSettings=(name,version,open)=>`<h2 id="language-label"></h2><div class="segments" role="group" aria-labelledby="language-label"><button data-language="system"></button><button data-language="zh">简体中文</button><button data-language="en">English</button></div><div class="about"><span>${name} ${version}</span><a id="github" href="https://github.com/Hezric/${name}" target="_blank" rel="noreferrer">GitHub ${open}</a></div>`;
export const pauseSettings=()=>`<h2 id="pause-for-label"></h2><div class="pause-row"><div class="segments presets" role="group" aria-labelledby="pause-for-label"><button data-minutes="15">15</button><button data-minutes="30">30</button><button data-minutes="60">60</button><button data-minutes="120">120</button></div><input id="pause-minutes" class="pause-minutes" type="text" inputmode="numeric" maxlength="4" aria-labelledby="pause-for-label" aria-describedby="pause-error" autocomplete="off" spellcheck="false"><span id="minutes-unit" class="minutes-unit"></span></div><div id="pause-error-wrap" class="message" aria-hidden="true"><div><p id="pause-error" class="pause-error" role="alert"></p></div></div>`;
