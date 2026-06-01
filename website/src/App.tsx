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
    <main className="min-h-screen overflow-x-hidden bg-[linear-gradient(135deg,#075056_0%,#0b6970_45%,#5f8f63_100%)] text-[#122124]">
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
    <section className="overflow-hidden px-5 py-16 text-white sm:px-8 sm:py-20 lg:px-10 lg:py-24">
      <div className="mx-auto flex w-[calc(100vw-2.5rem)] min-w-0 max-w-4xl flex-col items-center text-center">
        <div className="rounded-[1.7rem] bg-white/95 p-2.5 shadow-2xl shadow-black/25">
          <img
            src="/hops.webp"
            alt="Hops logo"
            className="h-20 w-20 rounded-[1.25rem]"
          />
        </div>

        <h1 className="mt-9 text-5xl font-semibold leading-none tracking-tight text-white sm:text-6xl lg:text-7xl">
          Hops
        </h1>
        <p className="mt-6 w-[calc(100vw-2.5rem)] max-w-[18rem] text-xl font-semibold leading-8 text-[#e9eef7] sm:max-w-2xl sm:text-2xl">
          Choose which browser opens every link.
        </p>
        <p className="mt-4 w-[calc(100vw-2.5rem)] max-w-[18rem] text-base leading-8 text-[#aebbd1] sm:max-w-2xl sm:text-lg">
          A free Windows tray app that routes external links to the right
          browser based on rules you set up.
        </p>
        <div className="mt-10 flex w-[calc(100vw-2.5rem)] max-w-[18rem] flex-col gap-3 sm:max-w-md sm:flex-row sm:justify-center">
          <a
            href={releaseUrl}
            className="inline-flex min-h-12 min-w-0 flex-1 items-center justify-center gap-2 rounded-xl bg-white px-6 py-3 text-sm font-semibold text-[#075056] shadow-lg shadow-[#10292d]/15 transition hover:bg-[#eef5f1] focus:outline-none focus:ring-2 focus:ring-[#c9ddd4] focus:ring-offset-2 focus:ring-offset-[#075056]"
          >
            <FiDownload aria-hidden="true" />
            Download
          </a>
          <a
            href={githubUrl}
            className="inline-flex min-h-12 min-w-0 flex-1 items-center justify-center gap-2 rounded-xl bg-[#ff6a00] px-6 py-3 text-sm font-semibold text-white shadow-lg shadow-[#10292d]/15 transition hover:bg-[#f05f00] focus:outline-none focus:ring-2 focus:ring-[#ffd3ad] focus:ring-offset-2 focus:ring-offset-[#075056]"
          >
            <FiGithub aria-hidden="true" />
            View on GitHub
          </a>
        </div>
        <p className="mt-6 text-sm text-[#8fa0ba]">
          Free and open source · Windows
        </p>
      </div>
    </section>
  );
}

function ScreenshotGallery() {
  return (
    <section className="px-5 py-20 sm:px-8 lg:px-10">
      <div className="mx-auto max-w-7xl">
        <div className="flex flex-col justify-between gap-5 md:flex-row md:items-end">
          <div className="max-w-3xl">
            <p className="text-sm font-semibold uppercase tracking-[0.18em] text-[#d8f2e6]">
              Screenshots
            </p>
            <h2 className="mt-3 text-3xl font-semibold tracking-tight text-white sm:text-4xl">
              A small app for a specific job.
            </h2>
          </div>
          <a
            href={githubUrl}
            className="inline-flex items-center gap-2 text-sm font-semibold text-white hover:text-[#d8f2e6]"
          >
            Browse the source
            <FiArrowRight aria-hidden="true" />
          </a>
        </div>

        <div className="mt-10 grid gap-6 lg:grid-cols-2">
          {screenshots.map((screenshot) => (
            <article
              key={screenshot.title}
              className="overflow-hidden rounded-2xl border border-white/70 bg-white/90 shadow-xl shadow-[#075056]/10 backdrop-blur"
            >
              <img
                src={screenshot.src}
                alt={`Hops ${screenshot.title.toLowerCase()} screen`}
                className={`aspect-[16/10] w-full ${
                  screenshot.fit === "contain"
                    ? "bg-[#f7faf8] object-contain object-center p-4"
                    : "object-cover object-top"
                }`}
                loading="lazy"
              />
              <div className="p-5">
                <h3 className="text-lg font-semibold text-[#10292d]">
                  {screenshot.title}
                </h3>
                <p className="mt-2 text-sm leading-6 text-[#53686b]">
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
    <section className="px-5 py-20 sm:px-8 lg:px-10">
      <div className="mx-auto grid max-w-7xl gap-10 lg:grid-cols-[0.8fr_1.2fr]">
        <div>
          <p className="text-sm font-semibold uppercase tracking-[0.18em] text-[#d8f2e6]">
            Setup
          </p>
          <h2 className="mt-3 text-3xl font-semibold tracking-tight text-white sm:text-4xl">
            Install once, then let Windows send links through Hops.
          </h2>
        </div>
        <div className="grid gap-4">
          {steps.map((step, index) => (
            <div
              key={step}
              className="flex gap-4 rounded-2xl border border-white/70 bg-white/85 p-5 shadow-xl shadow-[#075056]/10 backdrop-blur"
            >
              <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-[#10292d] text-sm font-semibold text-white">
                {index + 1}
              </div>
              <p className="pt-1 text-base leading-7 text-[#3d5356]">{step}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function FaqSection() {
  return (
    <section className="px-5 py-20 sm:px-8 lg:px-10">
      <div className="mx-auto max-w-7xl">
        <div className="max-w-3xl">
          <p className="text-sm font-semibold uppercase tracking-[0.18em] text-[#d8f2e6]">
            FAQ
          </p>
          <h2 className="mt-3 text-3xl font-semibold tracking-tight text-white sm:text-4xl">
            Practical details before you install.
          </h2>
        </div>
        <div className="mt-10 grid gap-4 md:grid-cols-2">
          {faqs.map((faq) => (
            <article
              key={faq.question}
              className="rounded-2xl border border-white/70 bg-white/85 p-5 shadow-sm backdrop-blur"
            >
              <h3 className="text-base font-semibold text-[#10292d]">
                {faq.question}
              </h3>
              <p className="mt-3 text-sm leading-6 text-[#53686b]">
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
    <footer className="px-5 pb-12 sm:px-8 lg:px-10">
      <div className="mx-auto flex max-w-7xl flex-col items-center justify-between gap-5 rounded-[2rem] border border-white/20 bg-white/10 p-6 text-center text-white shadow-xl shadow-[#075056]/10 backdrop-blur sm:flex-row sm:text-left">
        <p className="text-sm text-[#d8f2e6]">
          Made by{" "}
          <a
            href={authorUrl}
            className="font-semibold text-white transition hover:text-[#ffd3ad]"
          >
            @hfa
          </a>
        </p>
        <nav className="flex flex-wrap items-center justify-center gap-3 text-sm font-semibold">
          <a
            href={websiteUrl}
            className="rounded-full border border-white/20 px-4 py-2 text-white transition hover:bg-white/10"
          >
            Website
          </a>
          <a
            href={authorUrl}
            className="rounded-full border border-white/20 px-4 py-2 text-white transition hover:bg-white/10"
          >
            X / Twitter
          </a>
          <a
            href={githubUrl}
            className="rounded-full bg-[#ff6a00] px-4 py-2 text-white transition hover:bg-[#f05f00]"
          >
            GitHub
          </a>
        </nav>
      </div>
    </footer>
  );
}

export default App;
