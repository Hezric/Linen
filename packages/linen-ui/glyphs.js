const glyphs=import.meta.glob('./glyphs/*.svg',{query:'?raw',import:'default',eager:true});
export const sharedGlyph=(name)=>glyphs[`./glyphs/${name}.svg`];
