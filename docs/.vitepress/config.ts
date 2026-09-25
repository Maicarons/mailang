import { defineConfig } from 'vitepress'
import versions from '../versions.json'

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
      { text: '入门', items: [
        { text: '介绍', link: prefix + '/guide/' },
        { text: '快速开始', link: prefix + '/guide/getting-started' },
      ]},
      { text: '语言特性', items: [
        { text: '语法指南', link: prefix + '/guide/syntax' },
        { text: '面向对象', link: prefix + '/guide/oop' },
      ]},
      { text: '生态系统', items: [
        { text: '标准库', link: prefix + '/guide/stdlib' },
        { text: '包注册表', link: prefix + '/guide/registry' },
        { text: 'FFI 接入', link: prefix + '/guide/ffi' },
        { text: 'WASM 集成', link: prefix + '/guide/wasm' },
        { text: 'IoT 部署', link: prefix + '/guide/iot' },
        { text: '真实硬件', link: prefix + '/guide/hardware' },
        { text: 'ESP32 点灯', link: prefix + '/guide/esp32-blink' },
        { text: '体积仪表盘', link: prefix + '/guide/footprint' },
      ]},
    ],
    [prefix + '/reference/']: [
      { text: '参考手册', items: [
        { text: '概述', link: prefix + '/reference/' },
        { text: '类型系统', link: prefix + '/reference/types' },
        { text: '运算符', link: prefix + '/reference/operators' },
        { text: '内置函数', link: prefix + '/reference/builtins' },
        { text: '错误码', link: prefix + '/reference/errors' },
      ]},
      { text: '编译原理', items: [
        { text: '编译器架构', link: prefix + '/reference/compiler' },
        { text: '解析器详解', link: prefix + '/reference/parser' },
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
  srcExclude: ["**/MODULE_SPEC.md", "**/compose/**"],
  srcExclude: ["**/MODULE_SPEC.md", "**/compose/**"],
  srcExclude: ["**/MODULE_SPEC.md", "**/compose/**"],
  srcExclude: ["**/MODULE_SPEC.md", "**/compose/**"],
  srcExclude: ["**/MODULE_SPEC.md", "**/compose/**"],
  srcExclude: ["**/MODULE_SPEC.md", "**/compose/**"],
  srcExclude: ["**/MODULE_SPEC.md", "**/compose/**"],
  vueTemplate: false,
  markdown: {
    config() {},
  },
  locales: {
    root: {
      label: '简体中文',
      lang: 'zh-CN',
      themeConfig: {
        nav: nav({ guide: '指南', reference: '参考', playground: 'Playground', github: 'GitHub', version: 'v' + latest }),
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
