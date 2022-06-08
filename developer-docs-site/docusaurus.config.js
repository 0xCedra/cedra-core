// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const lightCodeTheme = require("prism-react-renderer/themes/github");
const darkCodeTheme = require("prism-react-renderer/themes/dracula");

const codeInjector = require("./src/remark/code-injector");

/** @type {import("@docusaurus/types").Config} */
const config = {
  title: "Aptos Labs",
  tagline: "Developer Documentation",
  url: "https://aptos.dev/",
  baseUrl: "/",
  onBrokenLinks: "warn",
  onBrokenMarkdownLinks: "warn",
  favicon: "img/favicon.ico",
  organizationName: "aptos-labs", // Usually your GitHub org/user name.
  projectName: "aptos-core", // Usually your repo name.

  presets: [
    [
      "@docusaurus/preset-classic",
      /** @type {import("@docusaurus/preset-classic").Options} */
      ({
        docs: {
          routeBasePath: "/",
          sidebarPath: require.resolve("./sidebars.js"),
          sidebarCollapsible: false,
          editUrl: "https://github.com/aptos-labs/aptos-core/tree/main/developer-docs-site/",
          remarkPlugins: [codeInjector],
        },
        blog: false,
        theme: {
          customCss: require.resolve("./src/css/custom.css"),
        },
        gtag: {
          trackingID: "G-HVB7QFB9PQ",
        },
      }),
    ],
  ],

  themeConfig:
  /** @type {import("@docusaurus/preset-classic").ThemeConfig} */
    ({
      docs: {
        sidebar: {
          hideable: true,
          autoCollapseCategories: true,
        }
      },
      navbar: {
        title: "| Developer Network",
        logo: {
          alt: "Aptos Labs Logo",
          src: "img/aptos_word.svg",
          srcDark: "/img/aptos_word.svg",
        },
        items: [
          {
            type: 'doc',
            label: "Start Here",
            position: "left",
            docId: 'basics/basics-txns-states',
          },
          {
            type: 'dropdown',
            label: "Tutorials",
            position: "left",
            items: [
              {
                type: 'doc',
                label: "Your First DApp",
                docId: 'tutorials/first-dapp',
              },
              {
                type: 'doc',
                label: "Your First NFT",
                docId: 'tutorials/your-first-nft',
              },
              {
                type: 'doc',
                label: "Building Wallet Extension",
                docId: 'tutorials/building-wallet-extension',
              },
              {
                type: 'doc',
                label: "Your First Coin",
                docId: 'tutorials/fist-coin',
              },
              {
                label: "Node Tutorials",
                type: 'doc',
                docId: 'tutorials/full-node/run-a-fullnode',
              },
            ],
          },
          {
            type: 'dropdown',
            label: "Guides",
            position: "left",
            items: [
              {
                label: "Life of a Transaction",
                type: 'doc',
                docId: 'guides/basics-life-of-txn',
              },
              {
                label: "Interacting with Aptos Blockchain",
                type: 'doc',
                docId: 'guides/interfacing-with-the-blockchain',
              },
            ],
          },
          {
            type: 'dropdown',
            label: "Move",
            position: "left",
            items: [
              {
                label: "Move on Aptos",
                type: 'doc',
                docId: 'guides/move',
              },
              {
                label: "Your First Move Module",
                type: 'doc',
                docId: 'tutorials/first-move-module',
              },
            ],
          },
          {
            type: 'dropdown',
            label: "Nodes",
            position: "left",
            items: [
              {
                label: "Node Tutorials",
                type: 'doc',
                docId: 'tutorials/full-node/run-a-fullnode',
              },
              {
                label: "Node Liveness Criteria",
                type: 'doc',
                docId: 'reference/node-liveness-criteria',
              },
              {
                label: "Local Testnet, Devnet and Incentivized Testnet",
                type: 'doc',
                docId: 'tutorials/local-testnet-devnet-incentivized-testnet',
              },
              {
                label: "Incentivized Testnet",
                type: 'doc',
                docId: 'tutorials/validator-node/intro',
              },
            ],
          },
          {
            type: 'doc',
            label: "DApps",
            position: "left",
            docId: 'tutorials/first-dapp',
          },
          {
            type: 'doc',
            label: "NFT",
            position: "left",
            docId: 'tutorials/your-first-nft',
          },
          {
            type: 'doc',
            label: "Wallet",
            position: "left",
            docId: 'tutorials/building-wallet-extension',
          },
          {
            href: "https://fullnode.devnet.aptoslabs.com/spec.html#/",
            label: "API",
            position: "left",
          },
          {
            href: "https://github.com/aptos-labs/aptos-core/",
            label: "GitHub",
            position: "right",
          },
        ],
      },
      footer: {
        style: "dark",
        links: [
          {
            title: null,
            items: [
              {
                html: `
                  <a class="social-link" href="https://aptoslabs.com" target="_blank" rel="noopener noreferrer" title="Git">
                     <img class="logo" src="/img/aptos_word.svg" alt="Git Icon" />
                  </a>
                `
              },
            ],
          },
          {
            title: null,
            items: [
              {
                html: `
                <p class="emails">
                  If you have any questions, please contact us at </br>
                  <a href="mailto:info@aptoslabs.com" target="_blank" rel="noreferrer noopener">
                    info@aptoslabs.com
                  </a> or
                  <a href="mailto:press@aptoslabs.com" target="_blank" rel="noreferrer noopener">
                    press@aptoslabs.com
                  </a>
                </p>
              `,
              },
            ],
          },
          {
            title: null,
            items: [
              {
                html: `
                  <p class="right">
                    <nav class="social-links">
                        <a class="social-link" href="https://github.com/aptoslabs" target="_blank" rel="noopener noreferrer" title="Git">
                         <img class="icon" src="/img/socials/git.svg" alt="Git Icon" />
                        </a>
                        <a class="social-link" href="https://discord.gg/aptoslabs" target="_blank" rel="noopener noreferrer" title="Discord">
                          <img class="icon" src="/img/socials/discord.svg" alt="Discord Icon" />
                        </a>
                        <a class="social-link" href="https://twitter.com/aptoslabs/" target="_blank" rel="noopener noreferrer" title="Twitter">
                          <img class="icon" src="/img/socials/twitter.svg" alt="Twitter Icon" />
                        </a>
                        <a class="social-link" href="https://aptoslabs.medium.com/" target="_blank" rel="noopener noreferrer" title="Medium">
                          <img class="icon" src="/img/socials/medium.svg" alt="Medium Icon" />
                        </a>
                        <a class="social-link" href="https://www.linkedin.com/company/aptoslabs/" target="_blank" rel="noopener noreferrer" title="LinkedIn">
                          <img class="icon" src="/img/socials/linkedin.svg" alt="LinkedIn Icon" />
                        </a>
                    </nav>
                  </p>
              `,
              },
            ],
          },
        ],
      },
      prism: {
        theme: lightCodeTheme,
        darkTheme: darkCodeTheme,
        additionalLanguages: ["rust"],
      },
      algolia: {
        appId: 'HM7UY0NMLG',
        apiKey: '63c5819714b74e64977337e61a1e3ae6',
        indexName: 'aptos',
        contextualSearch: true,
        debug: false,
      },
    }),
  plugins: [
    [
      '@docusaurus/plugin-client-redirects',
      {
        redirects: [
          {
            to: '/tutorials/full-node/run-a-fullnode',
            from: '/tutorials/run-a-fullnode',
          },
        ],
      },
    ],
  ],
};

module.exports = config;