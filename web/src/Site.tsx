import { useEffect, useRef, useState } from "react";
import {
  ArrowRight,
  Route,
  ScanLine,
  FlaskConical,
  BookOpen,
  Menu,
  X,
  ArrowUpRight,
} from "lucide-react";
import { EngineDemo } from "./EngineDemo";
import { Documentation } from "./Documentation";
const repository = "https://github.com/404khai/roadrunner";
export function Site() {
  const [menuOpen, setMenuOpen] = useState(false);
  const header = useRef<HTMLElement>(null),
    menuButton = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!menuOpen) return;
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMenuOpen(false);
        menuButton.current?.focus();
      }
    };
    const outside = (event: PointerEvent) => {
      if (!header.current?.contains(event.target as Node)) setMenuOpen(false);
    };
    document.addEventListener("keydown", key);
    document.addEventListener("pointerdown", outside);
    return () => {
      document.removeEventListener("keydown", key);
      document.removeEventListener("pointerdown", outside);
    };
  }, [menuOpen]);
  const docs = window.location.pathname.startsWith("/docs");
  useEffect(() => {
    document.title = docs
      ? "Documentation · Roadrunner"
      : "Roadrunner · Every delivery. A better way there.";
  }, [docs]);
  return (
    <div className="site">
      <a className="skip-content" href="#content">
        Skip to content
      </a>
      <header className="site-header" ref={header}>
        <a className="brand" href="/" aria-label="Roadrunner home">
          <img src="/roadrunner-mark.png" alt="" /> roadrunner
        </a>
        <button
          ref={menuButton}
          className="menu-toggle"
          type="button"
          aria-label={menuOpen ? "Close navigation" : "Open navigation"}
          aria-expanded={menuOpen}
          aria-controls="site-navigation"
          onClick={() => setMenuOpen((open) => !open)}
        >
          {menuOpen ? <X size={23} /> : <Menu size={23} />}
        </button>
        <nav
          id="site-navigation"
          aria-label="Main navigation"
          data-open={menuOpen}
          onClick={() => setMenuOpen(false)}
        >
          <a href="/#engine">The engine</a>
          <a href="/#how-it-works">How it works</a>
          <a href="/docs" aria-current={docs ? "page" : undefined}>
            Documentation
          </a>
          <a className="site-button compact mobile-workspace" href="/workspace">
            <Route size={16} /> Open workspace
          </a>
        </nav>
        <a className="site-button compact desktop-workspace" href="/workspace">
          Open workspace <ArrowUpRight size={15} />
        </a>
      </header>
      {docs ? <Documentation /> : <Landing />}
      <footer className="site-footer">
        <a className="brand" href="/">
          <img src="/roadrunner-mark.png" alt="" /> roadrunner
        </a>
        <span>Routing, dispatch & delivery simulation.</span>
        <a className="footer-icon-link" href={repository}>
          <GitHubMark size={16} /> GitHub
        </a>
        <a className="footer-icon-link" href="/docs">
          <BookOpen size={16} aria-hidden="true" /> Documentation
        </a>
      </footer>
    </div>
  );
}
function Landing() {
  return (
    <main id="content">
      <section className="site-hero">
        <p className="eyebrow">
          <span className="status-dot" /> BUILT IN RUST. GROUNDED IN THE REAL
          WORLD.
        </p>
        <h1>
          Every delivery.
          <br />A better way there.
        </h1>
        <p className="hero-description">
          From the road beneath a rider to the next order in their bag.
          <br className="desktop-break" /> Routing, dispatch, and simulation in
          one explainable last-mile engine.
        </p>
        <div className="site-actions">
          <a className="site-button" href="#demo">
            Explore the engine <ArrowUpRight size={17} />
          </a>
          <a className="site-button secondary" href="/docs">
            Read the docs <ArrowRight size={17} />
          </a>
        </div>
      </section>
      <div className="site-container">
        <EngineDemo />
      </div>
      <section className="site-section" id="engine">
        <div className="section-heading">
          <div>
            <p className="eyebrow">THE ENGINE / THREE CONNECTED LAYERS</p>
            <h2>
              A whole delivery system.
              <br />
              Every decision connected.
            </h2>
          </div>
          <p>
            Roads, riders, and restaurant readiness.
            <br />
            Model them together to understand
            <br />
            what makes a delivery work.
          </p>
        </div>
        <div className="feature-grid">
          {[
            {
              icon: Route,
              title: "Route with the road.",
              text: "Dijkstra and A*, implemented in Rust. Legal maneuvers, diverse alternatives, and explicit traffic costs on OSM road networks.",
              tag: "DIJKSTRA / A* / OSM",
              mode: "routing",
            },
            {
              icon: ScanLine,
              title: "Dispatch with context.",
              text: "Inspect eligibility, pickup timing, kitchen readiness, and candidate scores. See which recommendation the engine retained and why.",
              tag: "READINESS / CAPACITY / SLA",
              mode: "dispatch",
            },
            {
              icon: FlaskConical,
              title: "Test the whole journey.",
              text: "Run deterministic delivery scenarios. Follow original routes, inspect accepted events, and replay the same evidence.",
              tag: "SIMULATE / REPLAY / COMPARE",
              mode: "delivery",
            },
          ].map(({ icon: Icon, ...f }) => (
            <a
              className="feature"
              href={"/workspace?mode=" + f.mode}
              key={f.mode}
            >
              <Icon size={48} strokeWidth={1} />
              <h3>{f.title}</h3>
              <p>{f.text}</p>
              <span className="eyebrow">{f.tag} ↗</span>
            </a>
          ))}
        </div>
      </section>
      <section className="journey" id="how-it-works">
        <div>
          <p className="eyebrow">HOW IT WORKS / FOLLOW THE ORDER</p>
          <h2>
            Good deliveries
            <br />
            start before
            <br />
            the first turn.
          </h2>
          <p>
            A nearby rider is only part of the picture. Inspect the kitchen, the
            roads, and the orders already on board.
          </p>
          <a href="/workspace?mode=dispatch">Explore dispatch decisions ↗</a>
        </div>
        <ol>
          {[
            [
              "ORDER CREATED",
              "An order enters the network.",
              "Pickup, destination, readiness, capacity, and deadline.",
            ],
            [
              "CANDIDATES EVALUATED",
              "The engine weighs the options.",
              "Explicit eligibility, legal routes, and retained candidate scores.",
            ],
            [
              "EXECUTION OBSERVED",
              "A plan, with a reason.",
              "Publication is distinct from execution. Accepted events establish arrival, service, and completion.",
            ],
          ].map(([tag, title, text], i) => (
            <li key={tag}>
              <span className="journey-number">0{i + 1}</span>
              <div>
                <p className="eyebrow">{tag}</p>
                <h3>{title}</h3>
                <p>{text}</p>
              </div>
            </li>
          ))}
        </ol>
      </section>
      <section className="site-section">
        <div className="section-heading">
          <div>
            <p className="eyebrow">OPEN THE BLACK BOX</p>
            <h2>Built to be understood.</h2>
          </div>
          <p>Keep the evidence close to the picture.</p>
        </div>
        <div className="bento">
          <a className="bento-large" href="/docs#simulation">
            <img
              src="/docs/engine-immersive.png"
              alt="Roadrunner delivery workspace showing engine routes and rider inspection"
              loading="lazy"
            />
            <div>
              <span className="eyebrow">01 / EXECUTION</span>
              <h3>Follow the delivery. Inspect the evidence.</h3>
              <p>
                Rider identity, custody, next stops, and accepted execution
                stages remain connected through playback.
              </p>
            </div>
          </a>
          <a href="/docs#dispatch">
            <img
              src="/docs/engine-dispatch-perspective.png"
              alt="Dispatch workspace with candidate routes and score inspection"
              loading="lazy"
            />
            <div>
              <span className="eyebrow">02 / DECISIONS</span>
              <h3>A recommendation you can explain.</h3>
              <p>
                Candidate costs and rejection reasons, without invented
                evaluation chronology.
              </p>
            </div>
          </a>
          <a href="/docs#routing">
            <img
              src="/docs/engine-route-alternatives.png"
              alt="Roadrunner alternative route comparison with original geometry and route ranks"
              loading="lazy"
            />
            <div>
              <span className="eyebrow">03 / ROUTES</span>
              <h3>More than one way there.</h3>
              <p>
                Compare the retained alternatives. Inspect distance, predicted
                time, and the route the engine selected.
              </p>
            </div>
          </a>
          <a className="bento-code" href="/docs#api">
            <BookOpen size={30} />
            <span className="eyebrow">04 / BUILD WITH IT</span>
            <h3>
              From your first route
              <br />
              to the HTTP contract.
            </h3>
            <code>
              POST /v1/routes
              <br />
              POST /v1/visualization/deliveries
              <br />
              POST /v1/visualization/dispatch
            </code>
            <span>Explore the API reference ↗</span>
          </a>
        </div>
      </section>
      <section className="site-cta">
        <p className="eyebrow">FOR THE PEOPLE BUILDING LAST-MILE SYSTEMS</p>
        <h2>
          Get closer to
          <br />
          the delivery problem.
        </h2>
        <p>
          Explore the algorithms. Run a scenario. Follow an order.
          <br />
          Start with the code and see how the pieces fit.
        </p>
        <div className="site-actions">
          <a className="site-button" href={repository}>
            <GitHubMark size={18} /> View on GitHub
          </a>
          <a className="site-button secondary" href="/docs">
            <BookOpen size={18} aria-hidden="true" /> Read the documentation
          </a>
        </div>
        <p className="eyebrow">
          RUST WORKSPACE · CLI · HTTP API · LOCAL SIMULATION
        </p>
      </section>
    </main>
  );
}

function GitHubMark({ size }: { size: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
      className="github-mark"
    >
      <path d="M12 .297C5.37.297 0 5.67 0 12.297c0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.043-1.61-4.043-1.61-.546-1.387-1.333-1.756-1.333-1.756-1.09-.745.083-.729.083-.729 1.205.084 1.838 1.237 1.838 1.237 1.07 1.835 2.809 1.305 3.495.998.108-.776.418-1.305.762-1.605-2.665-.305-5.467-1.334-5.467-5.931 0-1.31.467-2.381 1.235-3.221-.123-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23a11.52 11.52 0 0 1 3.003-.404c1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.654 1.652.242 2.873.119 3.176.77.84 1.233 1.911 1.233 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222 0 1.606-.015 2.898-.015 3.293 0 .322.216.694.825.576C20.565 22.092 24 17.597 24 12.297c0-6.627-5.373-12-12-12" />
    </svg>
  );
}
