addEventListener('DOMContentLoaded', async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  try {
    const style = document.createElement('style');
    style.textContent = window.probeStyle;
    document.head.append(style);
    document.documentElement.dataset.theme = window.probeTheme;
    document.body.className = 'native';
    document.body.innerHTML = '<main id="app"><section class="flyout"><div class="current" style="display:none"></div><div id="views"><section style="height:278px"></section></div><div id="error-wrap" style="display:none"></div></section></main>';
    const panel = document.querySelector('.flyout');
    const content = document.querySelector('#views section');
    const frames = async () => {
      await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
    };
    const record = async stage => {
      await frames();
      if (window.probeMode === 'fallback') {
        const expected = window.probeTheme === 'dark' ? 'rgb(28, 28, 28)' : 'rgb(238, 238, 238)';
        if (getComputedStyle(panel).backgroundColor !== expected) throw Error('Opaque fallback was not rendered');
      }
      if (window.probeMode === 'material' && panel.dataset.backdrop !== 'true') throw Error('Material disappeared during transition');
      const r = panel.getBoundingClientRect();
      await invoke('record_frame', { stage, panel: [r.left, r.top, r.right, r.bottom] });
    };
    const { fitPanel } = await import(URL.createObjectURL(new Blob([window.probeSource], { type: 'text/javascript' })));
    let phase = '', frame = 0;
    const fit = fitPanel(document, async (command, args) => {
      // Observe the DOM frame before its matching native clip update can arrive.
      if (phase && args.reserve === undefined && !args.settled) await record(`${phase}-frame-${frame++}`);
      const layout = await invoke(command, args);
      if (window.probeMode === 'material' && args.settled && !layout.backdrop) throw Error('Native material unavailable');
      return layout;
    });
    await fit();
    await invoke('show');
    await record('initial');
    document.body.classList.add('presented');
    for (const [name, height] of [['shrink', 158], ['grow', 278]]) {
      phase = name; frame = 0;
      content.style.height = `${height}px`;
      await fit();
      phase = '';
      await record(`${name}-settled`);
    }
    await invoke('finish');
  } catch (error) {
    await invoke('finish', { error: String(error.stack || error) });
  }
});
