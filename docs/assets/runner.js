// MaìLang inline code runner for documentation pages
// Include this script on any page to make code blocks with class "mai-runnable" executable.

let _maiInterpreter = null;
let _maiLoading = null;

async function _maiGetInterpreter() {
  if (_maiInterpreter) return _maiInterpreter;
  if (_maiLoading) return _maiLoading;

  _maiLoading = (async () => {
    const base = document.currentScript?.src?.replace(/\/assets\/runner\.js.*$/, '') || '.';
    const mod = await import(`${base}/wasm/mailang_wasm.js`);
    await mod.default();
    _maiInterpreter = new mod.WasmInterpreter();
    return _maiInterpreter;
  })();

  return _maiLoading;
}

function _maiEscapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

async function _maiRunBlock(pre, code) {
  const outputEl = pre.parentElement.querySelector('.mai-output');
  if (!outputEl) return;

  outputEl.style.display = 'block';
  outputEl.innerHTML = '<span style="color:#8b949e">运行中...</span>';

  try {
    const interp = await _maiGetInterpreter();
    interp.reset();
    const result = JSON.parse(interp.eval_json(code));
    if (result.ok) {
      outputEl.innerHTML = result.output
        ? `<span style="color:#3fb950">${_maiEscapeHtml(result.output)}</span>`
        : '<span style="color:#8b949e">(无输出)</span>';
    } else {
      outputEl.innerHTML = `<span style="color:#f85149">${_maiEscapeHtml(result.error)}</span>`;
    }
  } catch (e) {
    outputEl.innerHTML = `<span style="color:#f85149">WASM 错误: ${_maiEscapeHtml(e.message)}</span>`;
  }
}

function _maiInitRunners() {
  document.querySelectorAll('pre > code.mai-runnable, pre.mai-runnable > code').forEach((codeEl) => {
    const pre = codeEl.closest('pre');
    if (pre.dataset.maiInit) return;
    pre.dataset.maiInit = '1';

    // Wrap in a container
    const wrapper = document.createElement('div');
    wrapper.className = 'mai-runner';
    wrapper.style.cssText = 'margin:16px 0;border:1px solid #30363d;border-radius:8px;overflow:hidden;';
    pre.parentNode.insertBefore(wrapper, pre);
    wrapper.appendChild(pre);
    pre.style.margin = '0';
    pre.style.border = 'none';
    pre.style.borderRadius = '0';

    // Toolbar
    const toolbar = document.createElement('div');
    toolbar.style.cssText = 'display:flex;align-items:center;justify-content:space-between;padding:6px 12px;background:#161b22;border-bottom:1px solid #30363d;font-size:0.8em;color:#8b949e;';
    toolbar.innerHTML = '<span>MaìLang</span>';

    const btn = document.createElement('button');
    btn.textContent = '▶ 运行';
    btn.style.cssText = 'padding:4px 12px;border:1px solid #2ea043;border-radius:4px;background:#238636;color:#fff;font-size:0.9em;cursor:pointer;';
    btn.onclick = () => _maiRunBlock(pre, codeEl.textContent);
    toolbar.appendChild(btn);
    wrapper.insertBefore(toolbar, pre);

    // Output area
    const output = document.createElement('div');
    output.className = 'mai-output';
    output.style.cssText = 'display:none;padding:12px 16px;background:#0d1117;border-top:1px solid #30363d;font-family:JetBrains Mono,Fira Code,Consolas,monospace;font-size:0.85em;white-space:pre-wrap;min-height:1.5em;';
    wrapper.appendChild(output);
  });
}

// Auto-init on DOM ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', _maiInitRunners);
} else {
  _maiInitRunners();
}
