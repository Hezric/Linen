import {defineConfig} from 'vite-plus';
export default defineConfig({build:{lib:{entry:{ui:'packages/linen-ui/index.js',glyphs:'packages/linen-ui/glyphs.js'},formats:['es']}}});
