// Extract unit test cases from the original repo's test-cases.ts and emit a
// Rust test file proving fix-fn fidelity.
const fs = require('fs');
const src = fs.readFileSync(
  '/var/folders/5d/ylkbk1nx6mvgq8zgnsjf8lwr0000gn/T/opencode/vue-stream-markdown/test/markmend/preprocess/test-cases.ts',
  'utf8'
);

const categories = {
  code: ['code-inline', 'code-block', 'code-mixed'],
  strong: ['strong-asterisk', 'strong-underscore'],
  emphasis: ['emphasis-asterisk', 'emphasis-underscore'],
  delete: ['delete'],
  link: ['link', 'image'],
  inline_math: ['inline-math'],
  math: ['math'],
  table: ['table'],
  task_list: ['task-list'],
  footnote: ['footnote'],
  html: ['html'],
};
const fixFns = {
  code: 'fix_code', strong: 'fix_strong', emphasis: 'fix_emphasis', delete: 'fix_delete',
  link: 'fix_link', inline_math: 'fix_inline_math', math: 'fix_math', table: 'fix_table',
  task_list: 'fix_task_list', footnote: 'fix_footnote', html: 'fix_html',
};

function extractArray(body, name) {
  const idx = body.indexOf(`'${name}': [`);
  if (idx === -1) return [];
  let depth = 0;
  let i = body.indexOf('[', idx);
  const start = i;
  let inStr = false, strCh = '';
  for (; i < body.length; i++) {
    const c = body[i];
    if (inStr) {
      if (c === '\\') { i++; continue; }
      if (c === strCh) inStr = false;
      continue;
    }
    if (c === "'" || c === '"') { inStr = true; strCh = c; continue; }
    if (c === '[') depth++;
    else if (c === ']') {
      depth--;
      if (depth === 0) break;
    }
  }
  return body.slice(start, i + 1);
}

function parseCaseObjects(arrText) {
  // split top-level objects
  const objs = [];
  let depth = 0, cur = '', inStr = false, strCh = '';
  for (let i = 0; i < arrText.length; i++) {
    const c = arrText[i];
    if (inStr) {
      cur += c;
      if (c === '\\') { cur += arrText[i + 1]; i++; continue; }
      if (c === strCh) inStr = false;
      continue;
    }
    if (c === "'" || c === '"') { inStr = true; strCh = c; cur += c; continue; }
    if (c === '{') depth++;
    else if (c === '}') depth--;
    if (c === '{' && depth === 1) { cur = '{'; continue; }
    if (c === '}' && depth === 0) { objs.push(cur + '}'); cur = ''; continue; }
    cur += c;
  }
  return objs.filter(o => o.includes('input'));
}

function parseField(obj, field) {
  const m = obj.match(new RegExp(`\\b${field}:\\s*(?:'(.*?)'|\"(.*?)\")`, 's'));
  if (!m) return undefined;
  let v = m[1] ?? m[2];
  v = v.replace(/\\n/g, '\n').replace(/\\t/g, '\t').replace(/\\\\/g, '\\').replace(/\\'/g, "'").replace(/\\"/g, '"');
  return v;
}

const rst = (s) => JSON.stringify(s);

let out = `//! AUTO-GENERATED from vue-stream-markdown test fixtures (test-cases.ts).
//! Regenerate with \`node scripts/gen-tests.mjs\`.
//! @generated

use crate::fix::*;

`;
for (const [name, cats] of Object.entries(categories)) {
  const fnName = fixFns[name];
  for (const cat of cats) {
    const arrText = extractArray(src, cat);
    const cases = parseCaseObjects(arrText).map(o => ({ input: parseField(o, 'input'), expected: parseField(o, 'expected') })).filter(c => c.input !== undefined && c.expected !== undefined);
    const fn = `#[test]\nfn generated_${fnName}_${cat.replace('-', '_')}() {\n    let _opts = crate::preprocess::PreprocessOptions::default();\n`;
    let body = '';
    let n = 0;
    for (const c of cases) {
      const arg = fnName === 'fix_strong' ? `${fnName}(${rst(c.input)}, &_opts)` : `${fnName}(${rst(c.input)})`;
      body += `    assert_eq!(${arg}, ${rst(c.expected)}, "case ${n}");\n`;
      n++;
    }
    out += fn + body + '}\n\n';
  }
}
fs.writeFileSync('/Users/yexrob/Episodes/Projects/rsmarkdown-tui/crates/core/src/generated_tests.rs', out);
console.log('wrote', out.split('\n').filter(l => l.startsWith('    assert')).length, 'assertions');
