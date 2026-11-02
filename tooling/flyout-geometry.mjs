// Shared rendered coverage; native taskbar acquisition is covered by linen-tray.
export function geometryChecks(test, expect, snapshot) {
  for (const scale of [1, 1.25, 1.5, 2]) {
    test.describe(`open geometry at ${scale * 100}%`, () => {
      test.use({ deviceScaleFactor: scale, viewport: { width: 405, height: 620 }, reducedMotion: 'reduce' });
      test('visible stroke gaps survive work-area changes and reopening', async ({ page }) => {
        let limit = 575, queries = 0;
        const errors = [];
        page.on('pageerror', error => errors.push(error.message));
        await page.exposeFunction('geometryLayout', () => { queries++; return { limit }; });
        await page.addInitScript(state => {
          window.isTauri = true;
          let id = 0;
          window.__TAURI_INTERNALS__ = {
            transformCallback() { return ++id; },
            async invoke(command) {
              if (command === 'plugin:event|listen') return 1;
              if (command === 'snapshot') return state;
              if (command === 'saved_networks') return { names: [], current: null };
              if (command === 'resize_flyout') return window.geometryLayout();
              if (['prepare_flyout', 'present_flyout', 'dismiss_flyout'].includes(command)) return;
              throw Error(`Unexpected simulated command ${command}`);
            },
          };
        }, snapshot);
        const gaps = () => page.locator('.flyout').evaluate(element => {
          const r = element.getBoundingClientRect();
          return { right: innerWidth - r.right, bottom: innerHeight - r.bottom,
            height: r.height, shadow: getComputedStyle(element).boxShadow, scale: devicePixelRatio };
        });
        const expectGaps = async () => {
          const g = await gaps();
          expect(g.right).toBeCloseTo(13, 1);
          expect(g.bottom).toBeCloseTo(13, 1);
          expect(g.shadow).toContain('0px 0px 0px 1px');
          expect(g.scale).toBe(scale);
          return g;
        };
        await page.goto('/');
        await expect(page.locator('body')).toHaveClass(/presented/);
        await expect(page.locator('.current')).toBeVisible();
        const original = await expectGaps();
        const before = queries;
        await expect.poll(() => queries).toBeGreaterThan(before);
        limit = 180;
        await page.setViewportSize({ width: 405, height: 225 });
        await expect.poll(async () => (await gaps()).height).toBeLessThanOrEqual(180);
        await expectGaps();
        limit = 655;
        await page.setViewportSize({ width: 405, height: 700 });
        await expect.poll(async () => (await gaps()).height).toBeCloseTo(original.height, 0);
        await expectGaps();
        await page.locator('#gear').click();
        await expect(page.locator('#settings')).toBeVisible();
        await expectGaps();
        await page.reload();
        await expect(page.locator('body')).toHaveClass(/presented/);
        await expectGaps();
        expect(errors).toEqual([]);
        const screenshot = test.info().outputPath('visible-stroke-and-shadow.png');
        await page.screenshot({ path: screenshot });
        await test.info().attach('visible stroke (12 DIP), shadow excluded', { path: screenshot, contentType: 'image/png' });
      });
    });
  }
}

export function materialChecks(test, expect, snapshot) {
  for (const dark of [false, true]) {
    test(`native material and fallback (${dark ? 'dark' : 'light'})`, async ({ page }) => {
      let backdrop = true;
      await page.exposeFunction('materialLayout', () => ({ limit: 575, backdrop }));
      await page.addInitScript(state => {
        window.isTauri = true;
        let id = 0;
        window.__TAURI_INTERNALS__ = {
          transformCallback() { return ++id; },
          async invoke(command) {
            if (command === 'plugin:event|listen') return 1;
            if (command === 'snapshot') return state;
            if (command === 'saved_networks') return { names: [], current: null };
            if (command === 'resize_flyout') return window.materialLayout();
            if (['prepare_flyout', 'present_flyout', 'dismiss_flyout'].includes(command)) return;
            throw Error(`Unexpected simulated command ${command}`);
          },
        };
      }, { ...snapshot, dark, accent: '#008060', accentInk: '#ffffff' });
      await page.goto('/');
      const panel = page.locator('.flyout');
      await expect(page.locator('body')).toHaveClass(/presented/);
      await expect(panel).toHaveAttribute('data-backdrop', 'true');
      // The browser simulates a desktop behind the native host, not Windows blur.
      await page.addStyleTag({ content: 'body.native { background:linear-gradient(120deg,#718b9b,#aebfa1); }' });
      await page.screenshot({ path: test.info().outputPath('material.png') });
      backdrop = false;
      await expect(panel).toHaveAttribute('data-backdrop', 'false');
      await expect(panel).toHaveCSS('background-color', dark ? 'rgb(28, 28, 28)' : 'rgb(238, 238, 238)');
      await page.screenshot({ path: test.info().outputPath('fallback.png') });
      await page.emulateMedia({ forcedColors: 'active' });
      await expect(panel).toHaveCSS('background-color', 'rgb(255, 255, 255)');
    });
  }
}