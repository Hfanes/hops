import {
  FiArrowRight,
  FiDownload,
  FiGithub,
} from "react-icons/fi";

const releaseUrl = "https://github.com/Hfanes/hops/releases/latest";
const githubUrl = "https://github.com/Hfanes/hops";
const authorUrl = "https://x.com/hfa_dev";
const websiteUrl = "https://www.hfanes.com/";

type Screenshot = {
  title: string;
  description: string;
  src: string;
  fit?: "cover" | "contain";
};

type Faq = {
  question: string;
  answer: string;
};

const screenshots: Screenshot[] = [
  {
    title: "Browsers",
    description:
      "Review detected browsers, hide picker entries, and add manual browser paths.",
    src: "/browsers.webp",
  },
  {
    title: "Rules",
    description:
      "Create ordered rules that route matching URLs to specific browsers.",
    src: "/rules.webp",
  },
  {
    title: "Settings",
    description:
      "Register Hops, manage defaults, configure startup, and choose routing behavior.",
    src: "/settings.webp",
  },
  {
    title: "Picker",
    description:
      "Choose a browser only when automatic routing is not the right answer.",
    src: "/windowpicker.webp",
    fit: "contain",
  },
];

const faqs: Faq[] = [
  {
    question: "Is Hops a browser?",
    answer:
      "No. Hops receives external http and https links, applies your routing rules, and opens the selected browser.",
  },
  {
    question: "Why does it need Windows Default Apps setup?",
    answer:
      "Windows sends external links to the selected default handler. Setting Hops for http and https lets it receive links from other apps.",
  },
  {
    question: "Does it require admin rights?",
    answer:
      "No. Hops registers itself for the current user under HKCU, so setup and rollback stay local to your account.",
  },
  {
    question: "Can I undo the integration?",
    answer:
      "Yes. Switch http and https away from Hops in Windows Default Apps, then use Unregister Hops in the app settings.",
  },
];

function App() {
  return (
    <main className="site-shell">
      <Hero />
      <ScreenshotGallery />
      <InstallSteps />
      <FaqSection />
      <LinksSection />
    </main>
  );
}

function Hero() {
  return (
    <section className="hero site-section">
      <div className="hero-inner">
        <div className="hero-logo-card">
          <img
            src="/hops.webp"
            alt="Hops logo"
            className="hero-logo"
          />
        </div>

        <h1 className="hero-title">
          Hops
        </h1>
        <p className="hero-tagline">
          Choose which browser opens every link.
        </p>
        <p className="hero-description">
          A free Windows tray app that routes external links to the right
          browser based on rules you set up.
        </p>
        <div className="hero-actions">
          <a
            href={releaseUrl}
            className="hero-button primary"
          >
            <FiDownload aria-hidden="true" />
            Download
          </a>
          <a
            href={githubUrl}
            className="hero-button accent"
          >
            <FiGithub aria-hidden="true" />
            View on GitHub
          </a>
        </div>
        <p className="hero-meta">
          Free and open source · Windows
        </p>
      </div>
    </section>
  );
}

function ScreenshotGallery() {
  return (
    <section className="site-section">
      <div className="site-container">
        <div className="section-heading-row">
          <div className="section-copy">
            <p className="eyebrow">
              Screenshots
            </p>
            <h2 className="section-title">
              A small app for a specific job.
            </h2>
          </div>
          <a
            href={githubUrl}
            className="text-link"
          >
            Browse the source
            <FiArrowRight aria-hidden="true" />
          </a>
        </div>

        <div className="screenshot-grid">
          {screenshots.map((screenshot) => (
            <article
              key={screenshot.title}
              className="screenshot-card"
            >
              <img
                src={screenshot.src}
                alt={`Hops ${screenshot.title.toLowerCase()} screen`}
                className={`screenshot-image ${
                  screenshot.fit === "contain"
                    ? "contain"
                    : "cover"
                }`}
                loading="lazy"
              />
              <div className="card-copy">
                <h3 className="card-title">
                  {screenshot.title}
                </h3>
                <p className="card-description">
                  {screenshot.description}
                </p>
              </div>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}

function InstallSteps() {
  const steps = [
    "Download the latest Windows installer from GitHub Releases.",
    "Install Hops and complete the first-run onboarding.",
    "Register Hops, then set http and https to Hops in Windows Default Apps.",
  ];

  return (
    <section className="site-section">
      <div className="site-container install-grid">
        <div>
          <p className="eyebrow">
            Setup
          </p>
          <h2 className="section-title">
            Install once, then let Windows send links through Hops.
          </h2>
        </div>
        <div className="step-list">
          {steps.map((step, index) => (
            <div
              key={step}
              className="step-card"
            >
              <div className="step-number">
                {index + 1}
              </div>
              <p className="step-text">{step}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function FaqSection() {
  return (
    <section className="site-section">
      <div className="site-container">
        <div className="section-copy">
          <p className="eyebrow">
            FAQ
          </p>
          <h2 className="section-title">
            Practical details before you install.
          </h2>
        </div>
        <div className="faq-grid">
          {faqs.map((faq) => (
            <article
              key={faq.question}
              className="faq-card"
            >
              <h3 className="faq-title">
                {faq.question}
              </h3>
              <p className="faq-answer">
                {faq.answer}
              </p>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}

function LinksSection() {
  return (
    <footer className="site-footer">
      <div className="site-container footer-card">
        <p className="footer-credit">
          Made by{" "}
          <a
            href={authorUrl}
            className="footer-author"
          >
            @hfa
          </a>
        </p>
        <nav className="footer-links">
          <a
            href={websiteUrl}
            className="footer-link"
          >
            Website
          </a>
          <a
            href={authorUrl}
            className="footer-link"
          >
            X / Twitter
          </a>
          <a
            href={githubUrl}
            className="footer-link accent"
          >
            GitHub
          </a>
        </nav>
      </div>
    </footer>
  );
}

export default App;
