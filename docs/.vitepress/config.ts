import { defineConfig } from 'vitepress'

const versions = {
  latest: '0.3',
  versions: [{ label: '0.3', path: '/v0.3/', latest: true }],
}
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
      { text: '\u5165\u95e8', items: [
        { text: '\u4ecb\u7ecd', link: prefix + '/guide/' },
        { text: '\u5feb\u901f\u5f00\u59cb', link: prefix + '/guide/getting-started' },
      ]},
      { text: '\u8bed\u8a00\u7279\u6027', items: [
        { text: '\u8bed\u6cd5\u6307\u5357', link: prefix + '/guide/syntax' },
        { text: '\u9762\u5411\u5bf9\u8c61', link: prefix + '/guide/oop' },
      ]},
      { text: '\u751f\u6001\u7cfb\u7edf', items: [
        { text: '\u6807\u51c6\u5e93', link: prefix + '/guide/stdlib' },
        { text: '\u5305\u6ce8\u518c\u8868', link: prefix + '/guide/registry' },
        { text: 'FFI \u63a5\u5165', link: prefix + '/guide/ffi' },
        { text: 'WASM \u96c6\u6210', link: prefix + '/guide/wasm' },
        { text: 'IoT \u90e8\u7f72', link: prefix + '/guide/iot' },
        { text: '\u771f\u5b9e\u786c\u4ef6', link: prefix + '/guide/hardware' },
        { text: 'ESP32 \u70b9\u706f', link: prefix + '/guide/esp32-blink' },
        { text: '\u4f53\u79ef\u4eea\u8868\u76d8', link: prefix + '/guide/footprint' },
      ]},
    ],
    [prefix + '/reference/']: [
      { text: '\u53c2\u8003\u624b\u518c', items: [
        { text: '\u6982\u8ff0', link: prefix + '/reference/' },
        { text: '\u7c7b\u578b\u7cfb\u7edf', link: prefix + '/reference/types' },
        { text: '\u8fd0\u7b97\u7b26', link: prefix + '/reference/operators' },
        { text: '\u5185\u7f6e\u51fd\u6570', link: prefix + '/reference/builtins' },
        { text: '\u9519\u8bef\u7801', link: prefix + '/reference/errors' },
      ]},
      { text: '\u7f16\u8bd1\u539f\u7406', items: [
        { text: '\u7f16\u8bd1\u5668\u67b6\u6784', link: prefix + '/reference/compiler' },
        { text: '\u89e3\u6790\u5668\u8be6\u89e3', link: prefix + '/reference/parser' },
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
  srcExclude: ['**/MODULE_SPEC.md', '**/compose/**'],
  vueTemplate: false,
  markdown: {
    config() {},
  },
  locales: {
    root: {
      label: '\u7b80\u4f53\u4e2d\u6587',
      lang: 'zh-CN',
      themeConfig: {
        nav: nav({ guide: '\u6307\u5357', reference: '\u53c2\u8003', playground: 'Playground', github: 'GitHub', version: 'v' + latest }),
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
