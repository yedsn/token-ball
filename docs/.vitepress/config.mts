import { defineConfig } from 'vitepress'

const githubRepo = 'https://github.com/yedsn/token-ball'
const giteeRepo = 'https://gitee.com/hongxiaojian/token-ball'
const repoName = process.env.GITHUB_REPOSITORY?.split('/')[1] ?? 'token-ball'
const base = process.env.DOCS_BASE ?? (process.env.GITHUB_ACTIONS === 'true' ? `/${repoName}/` : '/')

export default defineConfig({
  title: 'TokenBall',
  description: '面向 AI 编程重度用户的桌面额度悬浮球。',
  lang: 'zh-CN',
  base,
  cleanUrls: true,
  lastUpdated: true,
  head: [
    ['link', { rel: 'icon', type: 'image/png', href: `${base}favicon.png` }],
    ['meta', { name: 'theme-color', content: '#22C55E' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:title', content: 'TokenBall' }],
    ['meta', { property: 'og:description', content: '一个面向 AI 编程重度用户的桌面额度悬浮球。' }]
  ],
  themeConfig: {
    logo: '/logo.png',
    nav: [
      { text: '首页', link: '/' },
      { text: '使用说明', link: '/guide/quick-start' },
      { text: '开发文档', link: '/develop/setup' },
      { text: 'GitHub', link: githubRepo }
    ],
    sidebar: {
      '/guide/': [
        {
          text: '使用说明',
          items: [
            { text: '快速开始', link: '/guide/quick-start' },
            { text: '核心概念', link: '/guide/core-concepts' },
            { text: '实例接入说明', link: '/guide/instance-onboarding' },
            { text: 'Provider 配置', link: '/guide/providers' }
          ]
        }
      ],
      '/develop/': [
        {
          text: '开发说明',
          items: [
            { text: '开发环境', link: '/develop/setup' },
            { text: '构建与发布', link: '/develop/build-and-release' }
          ]
        }
      ],
      '/reference/': [
        {
          text: '参考',
          items: [
            { text: '项目结构', link: '/reference/project-structure' },
            { text: '常用命令', link: '/reference/commands' },
            { text: '站点与发布说明', link: '/reference/site-deployment' },
            { text: '千问接口说明', link: '/qianwen-token-plan-api' }
          ]
        }
      ]
    },
    socialLinks: [
      { icon: 'github', link: githubRepo }
    ],
    search: {
      provider: 'local'
    },
    editLink: {
      pattern: `${githubRepo}/edit/master/docs/:path`,
      text: '在 GitHub 上编辑此页'
    },
    outline: {
      label: '页面导航'
    },
    docFooter: {
      prev: '上一页',
      next: '下一页'
    },
    lastUpdated: {
      text: '最后更新于'
    },
    footer: {
      message: `GitHub: ${githubRepo} · Gitee: ${giteeRepo}`,
      copyright: 'Copyright 2026 TokenBall Contributors'
    }
  }
})
