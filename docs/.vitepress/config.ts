import { defineConfig } from 'vitepress'
const versions = { latest: '0.3', versions: [{ label: '0.3', path: '/v0.3/', latest: true }] }

const latest = versions.latest
const versionItems = versions.versions.map((v: { label: string; path: string; latest?: boolean }) => ({
  text: v.latest ? v.label + ' (latest)' : v.label,
  link: v.path + 'guide/',
}))

function nav(t: { guide: string; reference: string; playground: string; github: string; version: string }) {
  return [
    { text: t.version, items: versionItems },
    { text: t.guide, link: '/' + latest + '/guide/' },
    { text: t.reference, link: '/' + latest + '/reference/' },
    { text: t.playground, link: '/' + latest + '/playground' },
    { text: t.github, link: 'https://github.com/Maicarons/mailang' },
  ]
}

function sidebarZh(prefix: string) {
  return {
    [prefix + '/guide/']: [
      { text: '鍏ラ棬', items: [
        { text: '浠嬬粛', link: prefix + '/guide/' },
        { text: '蹇€熷紑濮?, link: prefix + '/guide/getting-started' },
      ]},
      { text: '璇█鐗规€?, items: [
        { text: '璇硶鎸囧崡', link: prefix + '/guide/syntax' },
        { text: '闈㈠悜瀵硅薄', link: prefix + '/guide/oop' },
      ]},
      { text: '鐢熸€佺郴缁?, items: [
        { text: '鏍囧噯搴?, link: prefix + '/guide/stdlib' },
        { text: '鍖呮敞鍐岃〃', link: prefix + '/guide/registry' },
        { text: 'FFI 鎺ュ叆', link: prefix + '/guide/ffi' },
        { text: 'WASM 闆嗘垚', link: prefix + '/guide/wasm' },
        { text: 'IoT 閮ㄧ讲', link: prefix + '/guide/iot' },
        { text: '鐪熷疄纭欢', link: prefix + '/guide/hardware' },
        { text: 'ESP32 鐐圭伅', link: prefix + '/guide/esp32-blink' },
        { text: '浣撶Н浠〃鐩?, link: prefix + '/guide/footprint' },
      ]},
    ],
    [prefix + '/reference/']: [
      { text: '鍙傝€冩墜鍐?, items: [
        { text: '姒傝堪', link: prefix + '/reference/' },
        { text: '绫诲瀷绯荤粺', link: prefix + '/reference/types' },
        { text: '杩愮畻绗?, link: prefix + '/reference/operators' },
        { text: '鍐呯疆鍑芥暟', link: prefix + '/reference/builtins' },
        { text: '閿欒鐮?, link: prefix + '/reference/errors' },
      ]},
      { text: '缂栬瘧鍘熺悊', items: [
        { text: '缂栬瘧鍣ㄦ灦鏋?, link: prefix + '/reference/compiler' },
        { text: '瑙ｆ瀽鍣ㄨ瑙?, link: prefix + '/reference/parser' },
      ]},
    ],
  }
}

function sidebarEn(prefix: string) {
  return {
    [prefix + '/en/guide/']: [
      { text: 'Getting Started', items: [
        { text: 'Introduction', link: prefix + '/en/guide/' },
        { text: 'Getting Started', link: prefix + '/en/guide/getting-started' },
      ]},
      { text: 'Language Features', items: [
        { text: 'Syntax Guide', link: prefix + '/en/guide/syntax' },
        { text: 'Object-Oriented', link: prefix + '/en/guide/oop' },
      ]},
      { text: 'Ecosystem', items: [
        { text: 'Standard Library', link: prefix + '/en/guide/stdlib' },
        { text: 'Package Registry', link: prefix + '/en/guide/registry' },
        { text: 'FFI', link: prefix + '/en/guide/ffi' },
        { text: 'WASM', link: prefix + '/en/guide/wasm' },
        { text: 'IoT Deployment', link: prefix + '/en/guide/iot' },
        { text: 'Real Hardware', link: prefix + '/en/guide/hardware' },
        { text: 'ESP32 Blink', link: prefix + '/en/guide/esp32-blink' },
        { text: 'Footprint', link: prefix + '/en/guide/footprint' },
      ]},
    ],
    [prefix + '/en/reference/']: [
      { text: 'Reference', items: [
        { text: 'Overview', link: prefix + '/en/reference/' },
        { text: 'Type System', link: prefix + '/en/reference/types' },
        { text: 'Operators', link: prefix + '/en/reference/operators' },
        { text: 'Built-ins', link: prefix + '/en/reference/builtins' },
        { text: 'Error Codes', link: prefix + '/en/reference/errors' },
      ]},
      { text: 'Compiler Internals', items: [
        { text: 'Compiler', link: prefix + '/en/reference/compiler' },
        { text: 'Parser', link: prefix + '/en/reference/parser' },
      ]},
    ],
  }
}

const vPrefix = '/' + latest

export default defineConfig({
  title: 'MaLang',
  description: 'MaLang - A modern programming language for IoT and cross-platform development',
  base: '/mailang/',
  ignoreDeadLinks: true,
  srcExclude: ["**/MODULE_SPEC.md", "**/compose/**"],
  vueTemplate: false,
  markdown: {
    config() {},
  },
  locales: {
    root: {
      label: '绠€浣撲腑鏂?,
      lang: 'zh-CN',
      themeConfig: {
        nav: nav({ guide: '鎸囧崡', reference: '鍙傝€?, playground: 'Playground', github: 'GitHub', version: 'v' + latest }),
        sidebar: { ...sidebarZh(vPrefix), ...sidebarEn(vPrefix) },
      },
    },
    en: {
      label: 'English',
      lang: 'en',
      themeConfig: {
        nav: nav({ guide: 'Guide', reference: 'Reference', playground: 'Playground', github: 'GitHub', version: 'v' + latest }),
        sidebar: { ...sidebarEn(vPrefix), ...sidebarZh(vPrefix) },
      },
    },
  },
})
