import React from 'react';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import Heading from '@theme/Heading';

const FEATURES = [
  {
    icon: '🔒',
    title: 'Fully On-Device',
    desc: 'All signal processing, feature extraction, and model inference run in the native layer. No raw events, no feature vectors, no baseline data ever leave the device.',
  },
  {
    icon: '🧠',
    title: 'Phase 2 Autoencoder',
    desc: 'A per-user 32→16→8→16→32 autoencoder is trained on-device at enrollment. Captures joint feature correlations that a per-feature z-score alone misses.',
  },
  {
    icon: '🛡️',
    title: 'AES-256-GCM Encrypted',
    desc: 'Profile and model weights are encrypted at rest with keys from Android Keystore — hardware-backed on supported devices.',
  },
  {
    icon: '📦',
    title: 'Drop-in AAR',
    desc: 'Three-line integration via BehaviorGuardManager. Handles Keystore key generation, sensor registration, and encrypted state persistence automatically.',
  },
  {
    icon: '⚡',
    title: 'Rust Native Core',
    desc: 'Signal processing and autoencoder training run as libbehavior_guard.so via JNI. Enrollment training completes in under one second on a mid-range device.',
  },
  {
    icon: '📊',
    title: 'Confidence-Aware Scoring',
    desc: 'Every risk score includes a confidence value reflecting session signal richness. Tune your thresholds to remain lenient on short or signal-sparse sessions.',
  },
];

function HeroBanner() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <div className="hero-banner">
      <div className="hero-badge">
        <span>✦</span>&nbsp;v0.1.0 · GPL-3.0 License
      </div>
      <Heading as="h1" className="hero-title">
        {siteConfig.title}
      </Heading>
      <p className="hero-tagline">{siteConfig.tagline}</p>
      <div className="hero-buttons">
        <Link className="btn-primary" to="/docs/intro">
          Get Started →
        </Link>
        <Link
          className="btn-secondary"
          href="https://github.com/rukmaldias/BehaviorGuard">
          GitHub ↗
        </Link>
      </div>
    </div>
  );
}

function StatsBar() {
  return (
    <div className="stats-bar">
      <div className="stats-bar__inner">
        {[
          {value: '32', label: 'features / session'},
          {value: '5', label: 'enrollment sessions'},
          {value: '<1 s', label: 'autoencoder training'},
          {value: '~5 KB', label: 'model size'},
          {value: 'API 24+', label: 'Android support'},
        ].map(({value, label}) => (
          <div className="stat" key={label}>
            <span className="stat__value">{value}</span>
            <span className="stat__label">{label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function FeaturesSection() {
  return (
    <section className="features-section">
      <Heading as="h2" className="section-title">
        What&rsquo;s inside
      </Heading>
      <p className="section-subtitle">
        Six properties that make BehaviorGuard production-ready.
      </p>
      <div className="features-grid">
        {FEATURES.map(({icon, title, desc}) => (
          <div className="feature-card" key={title}>
            <span className="feature-card__icon">{icon}</span>
            <div className="feature-card__title">{title}</div>
            <p className="feature-card__desc">{desc}</p>
          </div>
        ))}
      </div>
    </section>
  );
}

function CtaStrip() {
  return (
    <div className="cta-strip">
      <Heading as="h2">Ready to integrate?</Heading>
      <p>
        Build the AAR in one command, then drop in three lines of Kotlin.
      </p>
      <div className="hero-buttons">
        <Link className="btn-primary" to="/docs/intro">
          Read the docs →
        </Link>
        <Link className="btn-secondary" to="/docs/api-reference">
          API Reference
        </Link>
      </div>
    </div>
  );
}

export default function Home(): JSX.Element {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout title={siteConfig.title} description={siteConfig.tagline}>
      <main>
        <HeroBanner />
        <StatsBar />
        <FeaturesSection />
        <CtaStrip />
      </main>
    </Layout>
  );
}
