addEventListener('DOMContentLoaded', async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  try {
    const style = document.createElement('style');
    style.textContent = window.probeStyle;
    document.head.append(style);
    document.documentElement.dataset.theme = window.probeTheme;
    document.body.className = 'native';
    document.body.style.background = 'transparent';
    document.body.innerHTML = '<main id="app"><section class="flyout"><div class="current" style="display:none"></div><div id="views"><section style="height:278px"></section></div><div id="error-wrap" style="display:none"></div></section></main>';
    const panel = document.querySelector('.flyout');
    const content = document.querySelector('#views section');
    const frames = async () => {
      await new Promise(requestAnimationFrame);
      await new Promise(requestAnimationFrame);
    };
    const capture = async stage => {
      await frames();
      const r = panel.getBoundingClientRect();
      await invoke('capture', { stage, panel: [r.left, r.top, r.right, r.bottom] });
    };
    const { fitPanel } = await import(URL.createObjectURL(new Blob([window.probeSource], { type: 'text/javascript' })));
    let phase = '', frame = 0;
    const fit = fitPanel(document, async (command, args) => {
      // Observe the DOM frame before its matching native clip update can arrive.
      if (phase && args.reserve === undefined && !args.settled) await capture(`${phase}-frame-${frame++}`);
      const layout = await invoke(command, args);
      if (['material', 'unlayered'].includes(window.probeMode) && args.settled && !layout.backdrop) throw Error('Native material unavailable');
      if (window.probeMode === 'transparent') layout.backdrop = true;
      return layout;
    });
    await fit();
    await invoke('show');
    await capture('initial');
    await invoke('capture', { stage: 'background', panel: [0, 0, 0, 0] });
    await frames();
    document.body.classList.add('presented');
    for (const [name, height] of [['shrink', 158], ['grow', 278]]) {
      phase = name; frame = 0;
      content.style.height = `${height}px`;
      await fit();
      phase = '';
      await capture(`${name}-settled`);
    }
    await invoke('finish');
  } catch (error) {
    await invoke('finish', { error: String(error.stack || error) });
  }
});
