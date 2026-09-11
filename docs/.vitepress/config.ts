import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'MaìLang',
  description: 'MaìLang - A modern programming language for IoT and cross-platform development',
  base: '/mailang/',
  ignoreDeadLinks: true,
  // Disable Vue template compilation in markdown to avoid {} conflicts
  vueTemplate: false,
  markdown: {
    // Configure markdown-it to not treat {} as Vue interpolation
    config(md) {
      // No special config needed when vueTemplate is false
    }
  },
  locales: {
    root: {
      label: '简体中文',
      lang: 'zh-CN',
      themeConfig: {
        nav: [
          { text: '指南', link: '/guide/' },
          { text: '参考', link: '/reference/' },
          { text: 'Playground', link: '/playground' },
          { text: 'GitHub', link: 'https://github.com/Maicarons/mailang' },
        ],
        sidebar: {
          '/guide/': [
            {
              text: '入门',
              items: [
                { text: '介绍', link: '/guide/' },
                { text: '快速开始', link: '/guide/getting-started' },
              ],
            },
            {
              text: '语言特性',
              items: [
                { text: '语法指南', link: '/guide/syntax' },
                { text: '面向对象', link: '/guide/oop' },
              ],
            },
            {
              text: '生态系统',
              items: [
                { text: '标准库', link: '/guide/stdlib' },
                { text: 'FFI 接入', link: '/guide/ffi' },
                { text: 'WASM 集成', link: '/guide/wasm' },
                { text: 'IoT 部署', link: '/guide/iot' },
              ],
            },
          ],
          '/reference/': [
            {
              text: '参考手册',
              items: [
                { text: '概述', link: '/reference/' },
                { text: '类型系统', link: '/reference/types' },
                { text: '运算符', link: '/reference/operators' },
                { text: '内置函数', link: '/reference/builtins' },
                { text: '错误码', link: '/reference/errors' },
              ],
            },
            {
              text: '编译原理',
              items: [
                { text: '编译器架构', link: '/reference/compiler' },
                { text: '解析器详解', link: '/reference/parser' },
              ],
            },
          ],
        },
      },
    },
    en: {
      label: 'English',
      lang: 'en',
      themeConfig: {
        nav: [
          { text: 'Guide', link: '/en/guide/' },
          { text: 'Reference', link: '/reference/' },
          { text: 'Playground', link: '/playground' },
          { text: 'GitHub', link: 'https://github.com/Maicarons/mailang' },
        ],
        sidebar: {
          '/en/guide/': [
            {
              text: 'Getting Started',
              items: [
                { text: 'Introduction', link: '/en/guide/' },
                { text: 'Getting Started', link: '/en/guide/getting-started' },
              ],
            },
            {
              text: 'Language Features',
              items: [
                { text: 'Syntax Guide', link: '/en/guide/syntax' },
                { text: 'Object-Oriented', link: '/en/guide/oop' },
              ],
            },
            {
              text: 'Ecosystem',
              items: [
                { text: 'Standard Library', link: '/en/guide/stdlib' },
                { text: 'FFI', link: '/en/guide/ffi' },
                { text: 'WASM', link: '/en/guide/wasm' },
                { text: 'IoT Deployment', link: '/en/guide/iot' },
              ],
            },
          ],
          '/reference/': [
            {
              text: 'Reference',
              items: [
                { text: 'Overview', link: '/reference/' },
                { text: 'Type System', link: '/reference/types' },
                { text: 'Operators', link: '/reference/operators' },
                { text: 'Built-ins', link: '/reference/builtins' },
                { text: 'Error Codes', link: '/reference/errors' },
              ],
            },
            {
              text: 'Compiler Internals',
              items: [
                { text: 'Compiler', link: '/en/reference/compiler' },
                { text: 'Parser', link: '/en/reference/parser' },
              ],
            },
          ],
        },
      },
    },
  },
})
