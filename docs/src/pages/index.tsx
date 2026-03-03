import type {ReactNode} from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import Heading from '@theme/Heading';

import styles from './index.module.css';

function HomepageHeader() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={clsx('hero hero--primary', styles.heroBanner)}>
      <div className="container">
        <Heading as="h1" className="hero__title">
          {siteConfig.title}
        </Heading>
        <p className="hero__subtitle">{siteConfig.tagline}</p>
        <div className={styles.buttons}>
          <Link
            className="button button--secondary button--lg"
            to="/docs/getting-started">
            Get Started →
          </Link>
        </div>
      </div>
    </header>
  );
}

function Feature({title, description}: {title: string; description: string}) {
  return (
    <div className={clsx('col col--4')}>
      <div className="text--center padding-horiz--md padding-vert--md">
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function Home(): ReactNode {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout
      title="Browser Automation Engine"
      description="High-performance browser automation engine written in Rust with native bindings for Node.js and Python.">
      <HomepageHeader />
      <main>
        <section className="container padding-vert--xl">
          <div className="row">
            <Feature
              title="80+ CLI Commands"
              description="Navigate, scrape, crawl, screenshot, and automate browsers from the command line. Stealth mode, network throttling, HAR recording, and more."
            />
            <Feature
              title="51 MCP Tools"
              description="Full AI agent integration via Model Context Protocol. Navigation, scraping, crawling, stealth, data processing, and automation namespaces."
            />
            <Feature
              title="HTTP REST API"
              description="Multi-instance Chrome management with accessibility-based element refs. PinchTab-style server with 18 endpoints."
            />
          </div>
          <div className="row">
            <Feature
              title="Node.js & Python SDKs"
              description="Native bindings via NAPI-RS and PyO3. 130+ methods for browser control, crypto, parsing, and storage."
            />
            <Feature
              title="Rust Performance"
              description="5.8MB release binary. 248 tests. 63 CDP modules. Built with chromiumoxide, axum, ring, and sled."
            />
            <Feature
              title="Stealth & Security"
              description="Anti-detection patches, fingerprint evasion, proxy rotation, WebAuthn/Passkey support, AES-256-GCM encryption."
            />
          </div>
        </section>
      </main>
    </Layout>
  );
}
