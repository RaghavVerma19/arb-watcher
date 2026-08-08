# Arb Watch — Design System & Product Design Document

> An AI-powered institutional arbitrage intelligence platform.
> Product: **Arb Watch** — a live scanner that watches Solana DEX pools,
> prices every token, and surfaces profitable N-leg arbitrage cycles the moment
> they appear.
>
> Design posture: **airy twilight.** A warm, breathable near-black with
> golden-light atmosphere instead of institutional blue-dark. The landing is
> editorial and cinematic (an Instrument Studio / Apple sensibility with a
> magazine serif voice); the terminal keeps its precision but softens into
> glass panels, generous whitespace, and buttery motion. Golden light and black
> ink is the palette personality — calm, warm, precise. Never neon, never
> gaming, never crypto-bro.

---

## 1. Design Philosophy

**Two environments, one product.** The landing is a story. The terminal is a
cockpit. The landing earns curiosity; the terminal earns trust.

### Core principles

1. **Whitespace is tension, not emptiness.** Negative space is used
   aggressively. Every section has room to exist. Nothing is ever crowded.
2. **One focal point per screen.** The landing shows exactly one metric. The
   terminal shows one dominant module. Users are never overwhelmed by a wall of
   equal-weight widgets.
3. **Typography is layout.** Hierarchy is created with size, weight, and
   tracking — never with borders. Headlines at 72px sit beside 13px labels;
   nothing is uniformly 16px.
4. **Premium comes from composition, not decoration.** No Orbitron, no neon,
   no glow piles. Luxury is restraint: hairline separators, generous gutters,
   almost-invisible elevation.
5. **Panels are carved into the page, not stacked on it.** Surfaces are
   translucent, sections separated by spacing and thin rules — not floating
   rectangles with shadows.
6. **Motion is cinematic.** Everything glides (300/500/700ms, opacity + blur +
   transform). Nothing snaps. Content fades in; it does not spin in.
7. **Accent is spent like money.** 99% grayscale, 1% accent. Green is semantic
   only — when it appears it is the loudest thing on screen.
8. **Speed is the message.** Live data updates in place: numbers count up,
   confidence bars move, latency updates, liquidity pulses.

### What this product is NOT

- Not TradingView, Binance, or Coinbase.
- Not a generic SaaS dashboard of identical cards.
- Not cyberpunk, not gamified, not a "web3 NFT" aesthetic.
- Not a wall of 20 statistics on the first screen.

---

## 2. The Two Environments

### 2.1 The Landing (emotional, editorial)

```
┌────────────────────────────────────────────────────────────┐
│ ◇ ARB WATCH            Overview · Terminal · Systems  [● Live] │  transparent nav
├────────────────────────────────────────────────────────────┤
│                                                            │
│  REAL-TIME ARBITRAGE                                       │  eyebrow (11px)
│  INTELLIGENCE                                              │  H1 (72px / 750)
│                                                            │
│  Finding market inefficiencies before everyone else.       │  sub (20px)
│                                                            │
│  +1.84%                                                   │  single floating metric
│  Best Opportunity Right Now                                │  (count-up, live)
│  Detected across six liquidity pools.                      │
│                                                            │
│  [ Enter Terminal → ]                                      │  CTA
│                                                            │
│  watching 6 pools · 5 tokens · 3 modes                     │  caption
└────────────────────────────────────────────────────────────┘
```

- Occupies ~the entire viewport.
- A quiet animated canvas behind: liquidity flowing between pools.
- One floating metric: the real best opportunity (or a calm
  "watching — no edge right now" state).
- Live: the WebSocket connects on load and drives the metric + route pulse.

### 2.2 The Terminal (functional, magazine)

A single asymmetric composition. One dominant module, two supporting, one
utility sidebar — never a grid of equals.

```
┌───────────────────────────────┬──────────────────────────────┐
│ OPPORTUNITY FEED (dominant)    │  EXECUTION (utility sidebar) │
│ live exchange identity         │  mission-control identity    │
│ count-up profit · conf bars    │  mode · tick · feed · rpc    │
│ latency · liquidity pulse      │  uptime · best edge          │
│ expandable rows (route+break)  │                              │
│ ─────────────────────────────  │                              │
│ EDGE HISTORY (analytical strip)│                              │
├───────────────────────────────┴──────────────────────────────┤
│ NETWORK SCULPTURE   │  MARKET INTELLIGENCE                    │
│ scientific identity │  research-paper identity                │
│ particles on routes │  prices · sparklines · liquidity        │
└─────────────────────┴─────────────────────────────────────────┘
```

The feed is the visual focus. It must feel alive: rows animate, profit counts
upward, confidence bars move, latency updates, liquidity pulses.

---

## 3. Color Tokens

**Personality: golden light on warm black.** The palette is twilight — a
warm, never-blue charcoal — lit by champagne-gold. Gold is the single accent
(interactive, focus, ambient light). Mint and rose carry profit/loss
(semantic only). Everything else is warm neutral.

| Token | Value | Role |
|---|---|---|
| `bg` | `#141109` | page background (warm charcoal) |
| `bg-2` | `#1A1711` | secondary backdrop (gradient stops) |
| `surface` | `rgba(29,25,18,0.72)` | glass base container (+ blur) |
| `raised` | `#262117` | chips, badges, hovered rows |
| `card` | `#211D15` | solid panel where glass can't read |
| `edge` | `rgba(250,240,220,0.10)` | hairline separators |
| `edge-strong` | `rgba(250,240,220,0.16)` | rules that must read |
| `text` | `#F8F2E7` | primary text (warm ivory) |
| `text-secondary` | `#C9BFA9` | labels, amounts, body copy |
| `muted` | `#9D937F` | placeholders, timestamps, weak data |
| `accent` | `#E8B04B` | gold — interactive, focus, the 1% accent |
| `accent-strong` | `#F6C764` | gold hover/bright moments |
| `accent-soft` | `rgba(232,176,75,0.14)` | gold washes (tags, selection) |
| `profit` | `#3ECF8E` | gains, positive delta, live status |
| `loss` | `#FB8E7D` | losses, negative delta |
| `warning` | `#F5A623` | degraded state (reconnecting) |
| `hover-overlay` | `rgba(250,240,220,0.05)` | row/card hover wash |
| `selection` | `rgba(232,176,75,0.22)` | text + row selection |
| `focus` | `#E8B04B` (2px ring) | keyboard focus indicator |

### Rules

- **Mint/rose are semantic only** — never decoration, never brand.
- **Gold is ~1% of the screen as accent**, but the *ambient light* (body
  aurora, card edge glow) may be golden generously — light is atmosphere, not
  decoration.
- Background: warm charcoal with two soft golden aurora glows (top-left faint,
  lower-right warmer) + a whisper of film grain. No visible gradients as
  objects; light, not shapes.
- Panels are warm glass: `surface` at ~70% + backdrop blur so the golden aura
  breathes behind them. Radii are generous (16–24px) so nothing feels
  machined.

---

## 4. Typography

### Family

Three voices, no more:

- **Fraunces** (variable, optical sizing) — the editorial display serif. Used
  only for the landing headline and module eyebrows in italic. Warm, soft, and
  unmistakably *human* — this is what pulls the product away from bot-face.
- **Manrope** — the UI sans. Rounded, calm, modern. Body, labels, buttons,
  module copy.
- **IBM Plex Mono** — every number, price, route figure, and telemetry
  readout. A real mono (tabular by default), never a sans masquerading as one.

Never more than these three families.

### Hierarchy (the ladder, not the wall)

| Step | Size / Line | Weight / Track | Use |
|---|---|---|---|
| caption | 11px / 16 · 500 · `0.08em` upper | section labels, eyebrows |
| label | 13px / 20 · 500 | supporting text, metadata |
| body | 15–16px / 26 · 400 | landing sub, terminal prose |
| H4 | 20px / 28 · 650 · `-0.01em` | module titles |
| H3 | 30px / 36 · 700 · `-0.02em` | section headers |
| H2 | 46px / 50 · 650 · `-0.02em` | secondary display |
| H1 | 68–80px / 78 · Fraunces 520 · `-0.02em` | landing headline |

Body line-height is generous (26px). Headings are tight. Hierarchy is obvious
at a glance.

### Number rules

- **Every number uses `font-variant-numeric: tabular-nums`.** No jumping digits
  on tick.
- **Prices, amounts, and route figures are IBM Plex Mono.** Symbols stay sans.
- Round-trip figures (`1,000 USDC → 1,013.24 USDC`) are mono, right-aligned.
- Large numbers read premium: big tabular digits, zero shimmer.

### Accessibility of type

- Body ≥ 13px; micro-labels ≤ 11px reserved for captions/eyebrows.
- Contrast: primary ≥ 12:1, secondary ≥ 7:1, muted ≥ 4.5:1 on their surfaces.
- The serif is display-only (never body) so it can't hurt legibility at small
  sizes.

---

## 5. Grid & Composition

| Tier | Columns | Max width | Margins |
|---|---|---|---|
| Desktop | 12 | 1320px (page 1440px) | 96px |
| Tablet | 8 | 1024px | 48px |
| Mobile | 4 | fluid | 24px |

### Composition rules

- **Landing:** single centered column, H1 aligned left, plenty of negative
  space; the canvas is the second half of the composition.
- **Terminal:** asymmetric magazine. The feed dominates (~7/12), the execution
  sidebar is a utility rail (~5/12 but narrow, ~300px), the supporting row
  splits into sculpture + intelligence. No two panels are the same width if
  they can be avoided.
- **Large section rhythm:** rows are separated by 72–120px of breathing room,
  not by borders.

---

## 6. Spacing

### Landing (luxurious)

```
40 · 64 · 96 · 128 · 160 · 200
```

### Terminal (airy data density)

9pt base within modules: **4, 9, 12, 18, 24, 32, 40, 48, 64, 80.**

| Context | Value |
|---|---|
| Page gutter (desktop) | 112px outer, 56px terminal |
| Section gap (landing) | 112–160px |
| Section gap (terminal) | 88–112px |
| Module padding | 40px (28px on dense lists) |
| Module-to-module gap | 24px |
| Table cell padding | 16px 20px (row ≈ 60px) |
| Chip padding | 10px 16px |
| Between rows | 0 (hairline-separated) |

Spaciousness is a feature: if a row feels cramped, give it air. Never invent
off-rhythm values.

---

## 7. Motion

### Durations & easing

| Duration | Use |
|---|---|
| 220ms | micro-interactions, row hover |
| 320ms | row expansion, tag changes |
| 600ms | module emergence, hover reveals |
| 800ms | view transitions, hero entrances |

Ease: `cubic-bezier(0.22, 1, 0.36, 1)` — a long, soft settle that starts
fast and coasts, the *buttery* curve. Larger motions (view switches) may use
`cubic-bezier(0.16, 1, 0.3, 1)` for an even longer coast. Allowed properties:
**opacity, blur, transform (translate/scale), color, box-shadow.** No bounce,
no elastic, no snap.

### Vocabulary

| Event | Motion |
|---|---|
| View switch (landing → terminal) | fade + blur + translate out (500ms) → cockpit sweep (300ms) → staggered module in (700ms) |
| New opportunity | row fades/translates in, profit counts up |
| Profit change | number counts up/down (500ms), color flash settles |
| Confidence change | bar eases to new width (500ms) |
| Connection live / lost | dot pulse; pill text change |
| Hover on route edge | edge brightens, particles accelerate (300ms) |
| Content load | fade-in with blur-to-sharp (700ms stagger) |

### Reduced motion

`@media (prefers-reduced-motion: reduce)` disables all animation: canvas
particles render one static frame, count-ups jump to their value, view
transitions are instant, pulses are static. Color still carries state.

---

## 8. Components

### 8.1 NavBar

- **Landing:** transparent, no border, 80px tall, large margins. Logo left,
  3 links center-left (Overview · Terminal · Systems), status right (mode tag,
  connection pill, RPC latency).
- **Terminal:** same bar, gains a hairline bottom rule and a translucent glass
  background.
- Links are text only (13px/500 secondary), hover brightens; active state gets
  the 1% accent.

### 8.2 Hero (landing)

- Eyebrow (11px uppercase, accent at 1%) → H1 (72px) → sub (20px) → **one**
  floating metric → CTA → caption line.
- Floating metric: `+x.xx%` at 48px mono, profit green, **count-up**, with a
  13px label ("Best Opportunity Right Now") and a muted detail line.
- No-opportunity state: metric becomes `—` with the label
  "Watching — no edge right now" and a calm detail line. Never an error.
- CTA: primary button "Enter Terminal →", 48px tall, radius 12px, accent bg on
  hover only (resting state is a refined outline/ghost so the accent stays 1%).

### 8.3 Network Sculpture (canvas)

- Nodes = tokens (circle, 1px ring, muted). Edges = pools (hairline).
- Particles flow along edges, tiny (1–2px) soft dots, low opacity — quiet.
- The **live best route** pulses: edges brighten to profit, particles
  accelerate and glow.
- Hover: node shows its price; edge shows pool + rate. Intelligence, not decor.
- Shared engine powers the landing background (dimmer, full-bleed) and the
  terminal module (prominent, interactive).

### 8.4 Opportunity Feed (dominant module)

- Live-exchange identity: dense tabular rows, live dots, flashes.
- Row anatomy: route badges (`SOL → USDC → ...`) · profit (`+1.32%` mono 20px,
  count-up; `132 bps` caption below) · confidence (`%` + animated bar) · exec
  (`max size USD · impact %`).
- New rows flash profit-green once; stable rows sit still.
- Expandable (`<details>`): route sculpture mini-diagram + profit breakdown bar
  + model explainer. Leg amounts mono, right-aligned.
- Empty state: calm, active ("watching N pools — new cycles flash in here").

### 8.5 Market Intelligence (supporting, research-paper)

- Editorial look: token symbol large (20px/600), price (24px mono), inline
  delta, sparkline, and footnote-style metadata (liquidity USD, DEX count,
  session %, volatility).
- Liquidity absorbed here: per-pool depth shown as a thin scientific bar with
  mono labels — a chart legend, not a widget.

### 8.6 Execution Sidebar (utility, mission-control)

- Telemetry readouts in mono rows: mode, tick, feed age, RPC, pools, scan
  notional, min profit, uptime. Best edge + confidence meter when present.
- Feels like instrument gauges: label left, tabular value right, hairline rows.

### 8.7 Edge History (analytical strip)

- Slim chart: mono axis labels, threshold dashed rule, area + line in profit
  green. No grid overload; labels only where they matter.

### 8.8 Status primitives

- **ModeTag:** pill 11px/600 uppercase; Simulator (amber), Live·Jupiter (blue),
  Mainnet (green). Static, never flashing.
- **ConnPill + dot:** 8px dot with soft pulse ring; emerald = streaming,
  amber = reconnecting; label always present.
- **Buttons:** heights 40/48/56; radius 12px; primary uses accent only on
  hover/focus; ghost outline. Focus ring 2px accent at +2px.
- **Badges:** semantic only (green/red/warning/neutral), 4px radius.

---

## 9. Interaction Rules

| Interaction | Behavior |
|---|---|
| Hover (interactive) | hover-overlay wash; sculpture edge brightens |
| Active / pressed | scale 0.98 on buttons; row state persists |
| Focus | visible 2px accent ring on every keyboard path |
| Expand row | click summary; chevron rotates; content fades/translates (300ms) |
| View switch | "Enter Terminal" and brand click glide between environments |
| Touch targets | ≥ 32px effective (rows/pills), ≥ 48px primary actions |

All interactive elements reachable and operable by keyboard (`<details>` is
natively keyboard-accessible).

---

## 10. Accessibility

1. **Contrast** — body ≥ 7:1; muted data ≥ 4.5:1 on its background.
2. **Focus** — every focusable element shows the accent ring; never suppressed.
3. **Keyboard** — full flow operable without a pointer.
4. **Screen readers** — live regions for the feed count and price strip;
   canvases are `aria-hidden` and paired with text summaries.
5. **Reduced motion** — see §7.
6. **Color independence** — profit/loss never conveyed by color alone; signs
   (`+`/`−`, `▲/▼`) accompany every colored figure.
7. **Tap targets** — ≥ 32px on mobile.
8. **Language / zoom** — no body text below 13px; layout survives 200% zoom.

---

## 11. Responsive

- **Desktop first** (≥ 1024px): full magazine composition.
- **Tablet (≤ 1024px):** supporting row stacks; margins 48px.
- **Mobile (≤ 640px):**
  - Landing H1 scales down fluidly (72px → clamp to ~44px); metric stays big.
  - Terminal stacks dominant-first: feed, exec, sculpture, intel.
  - Feed row collapses to route + profit; exec meta hides to a `<details>`.
  - Text never below 13px. Complexity is removed, not compressed.

---

## 12. Design Review Checklist

- [ ] Colors resolve to §3 tokens (no raw hex in components).
- [ ] Spacing ∈ §6 scale; radii ∈ {8, 12, 16, 20, 24, 999}.
- [ ] Numbers tabular; prices/amounts mono; signs explicit on profit/loss.
- [ ] Accent ≤ ~1% of any screen; green/red semantic only.
- [ ] One focal point per screen; landing shows exactly one metric.
- [ ] Focus rings visible on all keyboard paths.
- [ ] `prefers-reduced-motion` honored (canvas static, transitions instant).
- [ ] Contrast ≥ 7:1 body, ≥ 4.5:1 muted.
- [ ] Mobile: dominant-first stack, text ≥ 13px, tap targets ≥ 32px.
- [ ] Empty states present, calm, and accurate (pool count).
- [ ] New components added to §8 before implementation.
- [ ] No animation exceeds 700ms except looped state indicators (pulse).
- [ ] Build + typecheck clean; `npm run build` passes.

---

## 13. Future Scalability

- **Token-first.** Future layouts consume §3–§7 tokens; never fork values.
- **Layout slots before components.** A new panel is a layout decision first.
- **Add, don't decorate.** Paper PnL, positions, and charts reuse the
  institutional table anatomy (§8.4).
- **State parity.** Every async surface exposes connecting/live/degraded like
  ConnPill.
- **Performance is a design constraint.** In-place DOM updates and one canvas
  engine reused across environments; keep ticks cheap.
- **The terminal must never lose its magazine asymmetry** as panels multiply —
  hierarchy collapses when everything becomes equal-width cards.
