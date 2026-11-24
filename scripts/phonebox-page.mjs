import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';

const html = readFileSync(new URL('../integrations/phonebox/tokenball-widget.html', import.meta.url), 'utf8');
const page = { phonebox: 'pages', version: 1, pages: [{ title: 'TokenBall', icon: '◉', widgets: [
  { type: 'html', x: 0, y: 0, w: 12, h: 6, options: { html, transparent: true } }
] }] };
const content = JSON.stringify(page, null, 2) + '\n';
const targets = [new URL('../integrations/phonebox/tokenball-page.json', import.meta.url), new URL('../docs/public/integrations/phonebox/tokenball-page.json', import.meta.url)];
for (const target of targets) {
  if (process.argv.includes('--check')) {
    if (readFileSync(target, 'utf8') !== content) throw new Error('PhoneBox 模板未同步：' + fileURLToPath(target));
  } else {
    mkdirSync(dirname(fileURLToPath(target)), { recursive: true });
    writeFileSync(target, content);
  }
}
console.log(process.argv.includes('--check') ? 'PhoneBox 模板一致性检查通过' : 'PhoneBox 页面模板已生成');
