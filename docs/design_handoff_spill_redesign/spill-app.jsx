// Spill. — Real Design (Daylight Cork, applied)
// Production-grade transformation of the locked wireframes.
//
// Design system:
// - Type: Inter (display + body), Caveat (one handwritten wall-moment per screen)
// - Palette: Daylight Cork (cream paper + clay/sage/red/violet card accents)
// - Surfaces: paper bg with cork-fiber noise, panel with paper grain
// - Cards: matte color fill, dual shadow (ambient + directional), 8px corner
// - Pins, tape, and slight rotations reserved for HERO moments only
// - All UI chrome is utility-confident; voice lives in headings + empty states

(() => {
const { DCArtboard } = window;

// ───────────────────────────────────────────── design tokens
const T = {
  // type
  font: '"Inter", system-ui, -apple-system, "Segoe UI", sans-serif',
  hand: '"Caveat", cursive',
  // palette — Daylight Cork
  paper:   '#f3e8cf',
  paper2:  '#ecdcb8',
  panel:   '#fbf3df',
  panelHi: '#fff8e6',
  line:    '#d9c89e',
  line2:   '#c4ae7a',
  fg:      '#1f1812',
  fg2:     '#4a3d2e',
  muted:   '#86755a',
  // card accents
  c: {
    mood:  '#cf8a3f', moodIn:  '#e9a44e',
    well:  '#2f9469', wellIn:  '#3eb486',
    wrong: '#cf4f4f', wrongIn: '#e26565',
    act:   '#8757b6', actIn:   '#a173d0',
  },
  // shadows
  s1: '0 1px 0 rgba(74,52,20,0.06), 0 2px 6px rgba(74,52,20,0.06)',
  s2: '0 1px 0 rgba(74,52,20,0.08), 0 8px 18px -4px rgba(74,52,20,0.12)',
  s3: '0 2px 0 rgba(74,52,20,0.08), 0 16px 32px -8px rgba(74,52,20,0.18)',
  // focus
  focus: '0 0 0 3px rgba(207,79,79,0.20)',
};

// One-time CSS for paper texture + animation cues
if (typeof document !== 'undefined' && !document.getElementById('spill-real-css')) {
  const s = document.createElement('style');
  s.id = 'spill-real-css';
  s.textContent = `
    .sp-paper {
      background-color: ${T.paper};
      background-image:
        radial-gradient(circle at 15% 20%, rgba(132,98,52,0.04) 0 1px, transparent 2px),
        radial-gradient(circle at 65% 35%, rgba(132,98,52,0.045) 0 1px, transparent 2px),
        radial-gradient(circle at 85% 70%, rgba(132,98,52,0.04) 0 1px, transparent 2px),
        radial-gradient(circle at 25% 80%, rgba(132,98,52,0.05) 0 1px, transparent 2px),
        radial-gradient(circle at 45% 55%, rgba(132,98,52,0.035) 0 1px, transparent 2px),
        repeating-linear-gradient(115deg, rgba(180,140,80,0.03) 0 1px, transparent 1px 6px),
        repeating-linear-gradient(35deg,  rgba(180,140,80,0.025) 0 1px, transparent 1px 5px);
    }
    .sp-panel-grain {
      background-image:
        repeating-linear-gradient(0deg, rgba(60,40,20,0.018) 0 1px, transparent 1px 4px),
        repeating-linear-gradient(90deg, rgba(60,40,20,0.012) 0 1px, transparent 1px 4px);
    }
    .sp-card-grain {
      position: relative;
    }
    .sp-card-grain::after {
      content: ''; position: absolute; inset: 0; pointer-events: none;
      background:
        repeating-linear-gradient(0deg, rgba(255,255,255,0.02) 0 1px, transparent 1px 3px),
        repeating-linear-gradient(90deg, rgba(0,0,0,0.025) 0 1px, transparent 1px 3px);
      border-radius: inherit; mix-blend-mode: overlay;
    }
    @keyframes sp-pulse {
      0%, 100% { box-shadow: 0 0 0 0 rgba(47,148,105,0.5); }
      50%      { box-shadow: 0 0 0 6px rgba(47,148,105,0); }
    }
    .sp-live-dot {
      animation: sp-pulse 1.8s ease-out infinite;
      background: ${T.c.well}; border-radius: 50%;
    }
    @keyframes sp-caret { 0%, 49% { opacity: 1 } 50%, 100% { opacity: 0 } }
    .sp-caret { display: inline-block; width: 2px; height: 1em; background: ${T.c.wrong};
      vertical-align: -2px; margin-left: 1px; animation: sp-caret 1s steps(1) infinite; }
    .sp-tape {
      position: absolute; width: 64px; height: 18px;
      background: linear-gradient(180deg, rgba(255,240,180,0.7), rgba(245,220,140,0.85));
      border-left: 1px solid rgba(255,255,255,0.4);
      border-right: 1px solid rgba(0,0,0,0.06);
      box-shadow: 0 1px 2px rgba(0,0,0,0.08);
    }
    .sp-pin {
      position: absolute; width: 14px; height: 14px; border-radius: 50%;
      box-shadow: 0 1px 2px rgba(0,0,0,0.3), inset -2px -2px 3px rgba(0,0,0,0.25), inset 2px 2px 3px rgba(255,255,255,0.4);
    }
    .sp-scroll::-webkit-scrollbar { width: 6px; height: 6px; }
    .sp-scroll::-webkit-scrollbar-thumb { background: ${T.line}; border-radius: 3px; }
    .sp-scroll::-webkit-scrollbar-track { background: transparent; }
  `;
  document.head.appendChild(s);
}

// ───────────────────────────────────────────── primitives
const Logo = ({ size = 22 }) => (
  <div style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
    <svg width={size * 1.2} height={size * 1.2} viewBox="0 0 30 30" style={{ display: 'block' }}>
      <defs>
        <linearGradient id="splg" x1="0" x2="1" y1="0" y2="1">
          <stop offset="0" stopColor={T.c.wrong} />
          <stop offset="1" stopColor="#a83232" />
        </linearGradient>
      </defs>
      <g transform="rotate(-22 15 16)">
        <rect x="6" y="8" width="14" height="13" rx="2" fill={T.paper} stroke={T.fg} strokeWidth="1.6" />
        <path d="M20 11 q 4 0 4 4 q 0 4 -4 4" fill="none" stroke={T.fg} strokeWidth="1.6" />
        <ellipse cx="13" cy="8" rx="7" ry="1.4" fill={T.fg} />
      </g>
      <ellipse cx="22" cy="24" rx="6" ry="2" fill="url(#splg)" stroke={T.fg} strokeWidth="0.9" />
      <circle cx="17" cy="21" r="1.4" fill="url(#splg)" stroke={T.fg} strokeWidth="0.6" />
      <circle cx="26" cy="20" r="1" fill="url(#splg)" stroke={T.fg} strokeWidth="0.5" />
    </svg>
    <span style={{
      fontFamily: T.font, fontWeight: 800, fontSize: size, color: T.fg,
      letterSpacing: -1.2, lineHeight: 1, display: 'inline-flex', alignItems: 'baseline',
    }}>
      Spill
      <span style={{
        width: size * 0.18, height: size * 0.18, marginLeft: 2,
        background: T.c.wrong, borderRadius: '50%', transform: 'translateY(2px)',
      }} />
    </span>
  </div>
);

const Btn = ({ kind = 'ghost', accent = T.c.wrong, icon, children, style }) => {
  const variants = {
    primary: {
      background: `linear-gradient(180deg, ${accent} 0%, ${shade(accent, -8)} 100%)`,
      color: '#fff', border: `1px solid ${shade(accent, -16)}`,
      boxShadow: `inset 0 1px 0 rgba(255,255,255,0.25), 0 1px 0 rgba(74,52,20,0.1), 0 2px 6px ${accent}40`,
    },
    secondary: {
      background: T.panelHi, color: T.fg, border: `1px solid ${T.line}`,
      boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.6), 0 1px 0 rgba(74,52,20,0.06)',
    },
    ghost: {
      background: 'transparent', color: T.fg2, border: `1px solid ${T.line}`,
    },
    dashed: {
      background: 'transparent', color: T.muted, border: `1px dashed ${T.line2}`,
    },
  };
  return (
    <span style={{
      display: 'inline-flex', alignItems: 'center', gap: 6,
      padding: '6px 12px', borderRadius: 8,
      fontFamily: T.font, fontSize: 12.5, fontWeight: 600, lineHeight: 1,
      whiteSpace: 'nowrap', cursor: 'pointer',
      ...variants[kind], ...style,
    }}>
      {icon && <span style={{ display: 'inline-flex' }}>{icon}</span>}
      {children}
    </span>
  );
};

// Lighten/darken hex by % (-100..100)
function shade(hex, percent) {
  const c = hex.replace('#', '');
  const num = parseInt(c, 16);
  let r = (num >> 16) + Math.round(255 * percent / 100);
  let g = ((num >> 8) & 0xff) + Math.round(255 * percent / 100);
  let b = (num & 0xff) + Math.round(255 * percent / 100);
  r = Math.max(0, Math.min(255, r));
  g = Math.max(0, Math.min(255, g));
  b = Math.max(0, Math.min(255, b));
  return '#' + ((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1);
}

const Avatar = ({ k, color, size = 26, status, ring }) => (
  <span style={{
    position: 'relative', display: 'inline-flex',
    width: size, height: size, borderRadius: '50%',
    background: `linear-gradient(135deg, ${color || T.line} 0%, ${shade(color || T.line, -18)} 100%)`,
    border: `2px solid ${ring || T.panel}`,
    fontFamily: T.font, fontSize: size * 0.38, fontWeight: 700,
    color: '#fff', alignItems: 'center', justifyContent: 'center',
    boxShadow: '0 1px 2px rgba(0,0,0,0.12), inset 0 1px 0 rgba(255,255,255,0.15)',
    textTransform: 'lowercase', letterSpacing: -0.3, flex: '0 0 auto',
  }}>
    {k}
    {status && <span style={{
      position: 'absolute', bottom: -1, right: -1,
      width: size * 0.34, height: size * 0.34, borderRadius: '50%',
      background: status === 'ready' ? T.c.well
        : status === 'writing' ? T.c.mood
        : status === 'voting' ? T.c.act
        : T.muted,
      border: `1.6px solid ${T.panel}`,
    }} />}
  </span>
);

const Stack = ({ people, size = 26, ring }) => (
  <div style={{ display: 'inline-flex' }}>
    {people.map((u, i) => (
      <span key={u.k} style={{ marginLeft: i ? -8 : 0 }}>
        <Avatar k={u.k} color={u.color} size={size} status={u.status} ring={ring} />
      </span>
    ))}
  </div>
);

const Pill = ({ tone = 'neutral', accent, children, style }) => {
  const tones = {
    neutral: { bg: T.panelHi, fg: T.fg2, border: T.line },
    solid:   { bg: accent || T.c.wrong, fg: '#fff', border: shade(accent || T.c.wrong, -14) },
    soft:    { bg: `${accent || T.c.wrong}1f`, fg: accent || T.c.wrong, border: `${accent || T.c.wrong}55` },
    ghost:   { bg: 'transparent', fg: T.muted, border: T.line2 },
  };
  const v = tones[tone];
  return (
    <span style={{
      display: 'inline-flex', alignItems: 'center', gap: 5,
      padding: '3px 9px', borderRadius: 99,
      fontFamily: T.font, fontSize: 11, fontWeight: 600, lineHeight: 1.3,
      letterSpacing: 0.1, background: v.bg, color: v.fg,
      border: `1px solid ${v.border}`, whiteSpace: 'nowrap', ...style,
    }}>{children}</span>
  );
};

// Card sleeve — color block with paper-grain overlay + dual shadow
const Card = ({ accent, children, rotate = 0, style, onClick }) => (
  <div onClick={onClick} className="sp-card-grain" style={{
    position: 'relative', background: `linear-gradient(180deg, ${shade(accent, 4)} 0%, ${accent} 60%, ${shade(accent, -6)} 100%)`,
    borderRadius: 8, padding: '12px 14px', color: '#fff',
    fontFamily: T.font, fontSize: 13.5, lineHeight: 1.45, fontWeight: 500,
    boxShadow: `${T.s2}, inset 0 1px 0 rgba(255,255,255,0.18), inset 0 -1px 0 rgba(0,0,0,0.08)`,
    transform: rotate ? `rotate(${rotate}deg)` : undefined,
    cursor: onClick ? 'pointer' : 'default',
    ...style,
  }}>{children}</div>
);

const GifTile = ({ h = 70, label = 'GIF', kind = 'a' }) => {
  // procedural patterns so different GIFs feel distinct
  const patterns = {
    a: 'linear-gradient(135deg, #e6d2a6 0%, #d4b577 40%, #8a6a3a 100%)',
    b: 'radial-gradient(circle at 30% 40%, #f4cdb0, #a85a3a 70%)',
    c: 'linear-gradient(180deg, #4a7a8c 0%, #2a4a5c 100%)',
    d: 'conic-gradient(from 220deg at 60% 40%, #d6a878, #9e6a40, #5a3a22, #d6a878)',
    e: 'linear-gradient(45deg, #6a8a4a 0%, #c4b870 50%, #d4a050 100%)',
  };
  return (
    <div style={{
      height: h, borderRadius: 6, position: 'relative', overflow: 'hidden',
      background: patterns[kind] || patterns.a,
      boxShadow: 'inset 0 0 0 1px rgba(255,255,255,0.15), inset 0 -8px 16px rgba(0,0,0,0.18)',
    }}>
      <span style={{
        position: 'absolute', top: 4, left: 4,
        fontSize: 9, fontWeight: 700, color: '#fff', letterSpacing: 0.5,
        padding: '2px 5px', borderRadius: 3,
        background: 'rgba(0,0,0,0.45)', backdropFilter: 'blur(4px)',
        fontFamily: T.font,
      }}>{label}</span>
    </div>
  );
};

// ─── App shell ──────────────────────────────────────────
const TopBar = ({ board, phase, presence, right }) => (
  <div className="sp-panel-grain" style={{
    height: 56, flex: '0 0 auto', padding: '0 20px',
    display: 'flex', alignItems: 'center', gap: 14,
    borderBottom: `1px solid ${T.line}`,
    background: T.panel,
    boxShadow: '0 1px 0 rgba(255,255,255,0.4) inset, 0 -1px 0 rgba(0,0,0,0.04) inset',
  }}>
    <Logo size={18} />
    {board && <>
      <span style={{ width: 1, height: 22, background: T.line }} />
      <div style={{ minWidth: 0 }}>
        <div style={{ fontFamily: T.font, fontSize: 13.5, fontWeight: 600, color: T.fg, lineHeight: 1.1 }}>{board}</div>
        {phase && <div style={{ fontFamily: T.font, fontSize: 10.5, color: T.muted, lineHeight: 1.2, marginTop: 1 }}>{phase}</div>}
      </div>
    </>}
    <div style={{ flex: 1 }} />
    {presence}
    <div style={{ display: 'inline-flex', gap: 6 }}>{right}</div>
  </div>
);

const Frame = ({ children }) => (
  <div className="sp-paper" style={{
    width: '100%', height: '100%', borderRadius: 10,
    border: `1px solid ${T.line}`, overflow: 'hidden',
    display: 'flex', flexDirection: 'column',
    fontFamily: T.font, color: T.fg, boxSizing: 'border-box',
    boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.4)',
  }}>{children}</div>
);

const ColHeader = ({ name, count, color, sub }) => (
  <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, paddingBottom: 10, paddingLeft: 2 }}>
    <span style={{ width: 8, height: 8, borderRadius: 4, background: color, boxShadow: `0 0 0 3px ${color}22`, alignSelf: 'center' }} />
    <span style={{ fontWeight: 700, fontSize: 13.5, color: T.fg, letterSpacing: -0.1 }}>{name}</span>
    {sub && <span style={{ fontSize: 11, color: T.muted, fontStyle: 'italic' }}>{sub}</span>}
    <span style={{ marginLeft: 'auto', fontSize: 11, color: T.muted, fontWeight: 600,
      padding: '2px 7px', borderRadius: 99, background: T.panelHi, border: `1px solid ${T.line}`,
    }}>{count}</span>
  </div>
);

const Tile = ({ children, style, hi }) => (
  <div className="sp-panel-grain" style={{
    background: hi ? T.panelHi : T.panel,
    border: `1px solid ${T.line}`, borderRadius: 10,
    padding: 14, fontFamily: T.font, color: T.fg,
    boxShadow: T.s1, ...style,
  }}>{children}</div>
);

// Shared team set
const TEAM = [
  { k: 'na', name: 'Nat',   color: '#cf4f4f', status: 'ready' },
  { k: 'lu', name: 'Lucas', color: '#cf8a3f', status: 'writing' },
  { k: 'sa', name: 'Sam',   color: '#8757b6', status: 'writing' },
  { k: 'kt', name: 'Katie', color: '#2f9469', status: 'ready' },
];

// Inline tag chips for column controls
const ColAddCard = ({ accent, active, hint }) => active ? (
  <div className="sp-card-grain" style={{
    position: 'relative',
    background: `linear-gradient(180deg, ${shade(accent, 4)} 0%, ${accent} 70%)`,
    borderRadius: 8, padding: '12px 14px', marginBottom: 10,
    color: '#fff', boxShadow: `0 0 0 3px ${accent}33, ${T.s2}`,
  }}>
    <div style={{ fontFamily: T.font, fontSize: 13.5, lineHeight: 1.4 }}>
      finally fixed the deploy flake<span className="sp-caret" style={{ background: '#fff' }} />
    </div>
    <div style={{ marginTop: 10, display: 'flex', alignItems: 'center', gap: 6 }}>
      <span style={{
        fontSize: 11, padding: '4px 8px', borderRadius: 99, background: '#fff', color: accent,
        fontWeight: 700, display: 'inline-flex', alignItems: 'center', gap: 4,
      }}>＋ GIF</span>
      <span style={{
        fontSize: 11, padding: '4px 8px', borderRadius: 99,
        background: 'rgba(255,255,255,0.18)', color: '#fff', fontWeight: 500,
      }}>＋ tag</span>
      <div style={{ flex: 1 }} />
      <span style={{ fontSize: 10.5, color: 'rgba(255,255,255,0.75)' }}>esc</span>
      <span style={{
        fontSize: 11, padding: '4px 11px', borderRadius: 99,
        background: '#fff', color: accent, fontWeight: 700,
        boxShadow: '0 1px 2px rgba(0,0,0,0.15)',
      }}>pin it ↵</span>
    </div>
  </div>
) : (
  <div style={{
    border: `1.5px dashed ${accent}55`, borderRadius: 8, padding: 10, marginBottom: 10,
    display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6,
    color: accent, fontSize: 12, fontWeight: 600, cursor: 'pointer',
    background: `${accent}08`,
  }}>
    <span style={{ fontSize: 14, lineHeight: 1, fontWeight: 700 }}>＋</span>
    <span>{hint}</span>
  </div>
);

// Hidden draft pip — the privacy device
const HiddenDraft = ({ accent }) => (
  <div style={{
    height: 52, borderRadius: 8, marginBottom: 10,
    background: `repeating-linear-gradient(45deg, ${accent}14 0 8px, ${accent}06 8px 16px)`,
    border: `1px dashed ${accent}55`,
    display: 'flex', alignItems: 'center', justifyContent: 'center',
    color: accent, fontSize: 11, fontWeight: 600, fontStyle: 'italic',
    letterSpacing: 0.3,
  }}>· · ·  someone's draft  · · ·</div>
);

// ─── ① OVERVIEW ─────────────────────────────────────────
window.real_Overview = function () {
  const boards = [
    { t: 'Sprint 42 · platform', s: 'WRITING', sub: '2 of 4 ready · waiting on you', c: T.c.mood, hi: true, time: 'open · today' },
    { t: 'Design QA · w24',       s: 'VOTING',  sub: '1 vote left',                   c: T.c.act,  time: 'open · today' },
    { t: 'Platform · monthly',    s: 'LIVE',    sub: '4 people in the room',          c: T.c.well, time: 'now',         live: true },
    { t: 'Bug bash · 5/30',       s: 'SCHEDULED', sub: 'opens fri 10am',              c: T.muted, time: 'fri' },
  ];
  return (
    <DCArtboard id="real-overview" label="① overview" width={1240} height={760}>
      <Frame>
        <TopBar
          right={<>
            <Btn kind="secondary">history</Btn>
            <Btn kind="secondary" icon={<span>🔍</span>}>search</Btn>
            <Btn kind="primary">＋ new board</Btn>
          </>}
          presence={<Avatar k="na" color="#cf4f4f" size={28} />}
        />
        <div style={{ flex: 1, padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 360px', gap: 36, minHeight: 0, overflow: 'hidden' }}>
          {/* left: boards */}
          <div style={{ minWidth: 0 }}>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: 12 }}>
              <h1 style={{ margin: 0, fontFamily: T.font, fontSize: 32, fontWeight: 800, letterSpacing: -0.8, color: T.fg }}>
                Still pinned
              </h1>
              <span style={{ fontFamily: T.hand, fontSize: 22, color: T.muted, transform: 'rotate(-2deg)' }}>boards in motion</span>
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16, marginTop: 18 }}>
              {boards.map((b, i) => (
                <div key={i} style={{
                  position: 'relative', background: T.panel, borderRadius: 12,
                  border: `1px solid ${T.line}`, padding: 16,
                  boxShadow: b.hi ? `0 0 0 2px ${b.c}, ${T.s3}` : T.s1,
                  height: 156, display: 'flex', flexDirection: 'column',
                  transition: 'transform 120ms', cursor: 'pointer',
                }}>
                  <span style={{
                    position: 'absolute', top: -9, left: 14,
                    padding: '3px 9px', borderRadius: 99,
                    background: b.c, color: '#fff',
                    fontSize: 9.5, fontWeight: 800, letterSpacing: 0.8,
                    display: 'inline-flex', alignItems: 'center', gap: 5,
                    boxShadow: `0 2px 4px ${b.c}55`,
                  }}>
                    {b.live && <span className="sp-live-dot" style={{ width: 6, height: 6 }} />}
                    {b.s}
                  </span>
                  <div style={{ marginTop: 8, fontSize: 17, fontWeight: 700, color: T.fg, letterSpacing: -0.2 }}>{b.t}</div>
                  <div style={{ fontSize: 12.5, color: T.muted, marginTop: 4 }}>{b.sub}</div>
                  <div style={{ flex: 1 }} />
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <Stack people={TEAM} size={22} ring={T.panel} />
                    <span style={{ fontSize: 11, color: b.c, fontWeight: 700, letterSpacing: 0.3, textTransform: 'uppercase' }}>open →</span>
                  </div>
                </div>
              ))}
            </div>

            <div style={{ marginTop: 28 }}>
              <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700 }}>QUICK START</div>
              <div style={{ marginTop: 10, display: 'flex', gap: 8 }}>
                <Btn kind="primary">＋ standard retro</Btn>
                <Btn kind="secondary">＋ 4 Ls</Btn>
                <Btn kind="secondary">＋ custom columns</Btn>
                <Btn kind="dashed">from template</Btn>
              </div>
            </div>
          </div>

          {/* right: memory rail */}
          <div style={{ borderLeft: `1px solid ${T.line}`, paddingLeft: 24, display: 'flex', flexDirection: 'column', gap: 18, minHeight: 0 }}>
            <div>
              <div style={{ fontFamily: T.hand, fontSize: 26, color: T.fg, lineHeight: 1, transform: 'rotate(-1deg)' }}>still on the wall</div>
              <div style={{ fontSize: 11.5, color: T.muted, fontStyle: 'italic', marginTop: 2 }}>themes that keep coming back</div>
            </div>

            <Tile style={{ background: `${T.c.wrong}10`, border: `1px solid ${T.c.wrong}55` }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{ width: 8, height: 8, borderRadius: 4, background: T.c.wrong, boxShadow: `0 0 0 3px ${T.c.wrong}33` }} />
                <span style={{ fontSize: 10, letterSpacing: 1.2, color: T.c.wrong, fontWeight: 800 }}>RECURRING</span>
                <div style={{ flex: 1 }} />
                <span style={{ fontSize: 10, color: T.muted }}>since 4/16</span>
              </div>
              <div style={{ marginTop: 6, fontSize: 18, color: T.fg, fontWeight: 700, letterSpacing: -0.2 }}>"flaky CI"</div>
              <div style={{ fontSize: 12, color: T.muted, marginTop: 2 }}>3 boards in a row · 2 open actions</div>
              <div style={{ marginTop: 10, display: 'flex', gap: 6 }}>
                {['s40','s41','s42'].map((s, i) => (
                  <span key={s} style={{
                    fontSize: 10, padding: '3px 8px', borderRadius: 99,
                    background: i === 2 ? T.c.wrong : `${T.c.wrong}33`,
                    color: i === 2 ? '#fff' : T.c.wrong, fontWeight: 700,
                  }}>{s}</span>
                ))}
              </div>
            </Tile>

            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700 }}>OPEN ACTIONS</div>
                <Pill tone="ghost">4</Pill>
              </div>
              <div style={{ marginTop: 10, display: 'flex', flexDirection: 'column', gap: 9 }}>
                {[
                  { t: 'quarantine flake suite', who: 'lucas', when: 'fri 5/30', c: '#cf8a3f' },
                  { t: 'flake counter on deploy gate', who: 'sam', when: 'tue 6/4', c: '#8757b6' },
                  { t: 'own the onboarding doc', who: 'nat', when: 'mon 5/27', c: '#cf4f4f', overdue: true },
                  { t: 'demo retro process to product', who: 'kt', when: 'wed 6/5', c: '#2f9469' },
                ].map((a, i) => (
                  <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12.5 }}>
                    <span style={{
                      width: 14, height: 14, borderRadius: 3,
                      border: `1.5px solid ${a.overdue ? T.c.wrong : T.line2}`,
                      background: '#fff',
                    }} />
                    <span style={{ flex: 1, color: T.fg }}>{a.t}</span>
                    <Avatar k={a.who.slice(0,2)} color={a.c} size={18} />
                    <span style={{ fontSize: 10.5, color: a.overdue ? T.c.wrong : T.muted, fontWeight: a.overdue ? 700 : 500, minWidth: 50, textAlign: 'right' }}>{a.when}</span>
                  </div>
                ))}
              </div>
            </div>

            <div style={{ paddingTop: 16, borderTop: `1px dashed ${T.line}` }}>
              <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700 }}>RECENT</div>
              <div style={{ marginTop: 8, display: 'flex', flexDirection: 'column', gap: 6 }}>
                {[
                  { t: 'sprint 41', m: 'mixed', d: '5/14', c: T.c.mood },
                  { t: 'sprint 40', m: 'steady', d: '5/7', c: T.c.well },
                  { t: 'bug bash',  m: 'stormy', d: '4/30', c: T.c.wrong },
                ].map((r) => (
                  <div key={r.t} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: 12.5 }}>
                    <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                      <span style={{ width: 6, height: 6, borderRadius: 3, background: r.c }} />
                      <span style={{ color: T.fg }}>{r.t}</span>
                    </span>
                    <span style={{ color: T.muted, fontSize: 11 }}>{r.m} · {r.d}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </Frame>
    </DCArtboard>
  );
};

// ─── ② NEW BOARD ───────────────────────────────────────
window.real_NewBoard = function () {
  const templates = [
    { n: 'Standard', cols: ['mood','went well','went wrong','actions'], active: true, sub: 'recommended' },
    { n: '4 Ls', cols: ['liked','lacked','learned','longed for'] },
    { n: 'Custom', cols: ['user deck mode'], sub: 'design your own' },
  ];
  return (
    <DCArtboard id="real-newboard" label="② new board" width={1240} height={760}>
      <Frame>
        <TopBar board="New board" phase="set it up · takes 20 seconds"
          right={<>
            <Btn kind="secondary">cancel</Btn>
            <Btn kind="primary">pin it up ↵</Btn>
          </>}
        />
        <div style={{ flex: 1, padding: '28px 32px', display: 'grid', gridTemplateColumns: '1fr 360px', gap: 36, overflow: 'hidden' }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 26, minWidth: 0 }}>
            {/* Step 1 — name */}
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 10 }}>
                <StepNum n="1" /><span style={{ fontWeight: 700, fontSize: 13, color: T.fg }}>Name it</span>
                <span style={{ fontSize: 11, color: T.muted }}>auto-suggested from your last board</span>
              </div>
              <div style={{
                padding: '14px 18px', borderRadius: 10,
                background: T.panelHi, border: `2px solid ${T.c.wrong}`,
                boxShadow: T.focus, position: 'relative',
              }}>
                <div style={{ fontSize: 24, fontWeight: 700, color: T.fg, letterSpacing: -0.4 }}>
                  Sprint 42<span className="sp-caret" />
                </div>
              </div>
            </div>

            {/* Step 2 — template */}
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 10 }}>
                <StepNum n="2" /><span style={{ fontWeight: 700, fontSize: 13, color: T.fg }}>Pick a shape</span>
              </div>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12 }}>
                {templates.map((tpl, i) => (
                  <div key={tpl.n} style={{
                    cursor: 'pointer', position: 'relative',
                    padding: 14, borderRadius: 12,
                    background: tpl.active ? `${T.c.wrong}10` : T.panel,
                    border: `${tpl.active ? '2' : '1'}px solid ${tpl.active ? T.c.wrong : T.line}`,
                    boxShadow: tpl.active ? T.s2 : T.s1,
                  }}>
                    {tpl.active && (
                      <span style={{
                        position: 'absolute', top: 10, right: 10,
                        width: 18, height: 18, borderRadius: 9,
                        background: T.c.wrong, color: '#fff', fontSize: 11, fontWeight: 700,
                        display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                      }}>✓</span>
                    )}
                    <div style={{ fontSize: 15, fontWeight: 700, color: T.fg }}>{tpl.n}</div>
                    {tpl.sub && <div style={{ fontSize: 10.5, color: tpl.active ? T.c.wrong : T.muted, fontWeight: 600, marginTop: 2, letterSpacing: 0.4, textTransform: 'uppercase' }}>{tpl.sub}</div>}
                    <div style={{ marginTop: 10, display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                      {tpl.cols.map((c, j) => {
                        const accents = [T.c.mood, T.c.well, T.c.wrong, T.c.act];
                        const accent = accents[j % accents.length];
                        return (
                          <span key={c} style={{
                            fontSize: 10, padding: '3px 8px', borderRadius: 99,
                            background: `${accent}1c`, color: shade(accent, -10), fontWeight: 600,
                            border: `1px solid ${accent}44`,
                          }}>{c}</span>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Step 3 — rules */}
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 10 }}>
                <StepNum n="3" /><span style={{ fontWeight: 700, fontSize: 13, color: T.fg }}>House rules</span>
              </div>
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
                {[
                  { l: 'votes per person', v: '3', icon: '●●●' },
                  { l: 'top voted → action', v: '3 cards', icon: '★' },
                  { l: 'clustering', v: 'manual · on demand', icon: '◆' },
                  { l: 'reveal mode', v: 'when all marked ready', icon: '☉' },
                ].map((f) => (
                  <Tile key={f.l} style={{
                    display: 'flex', alignItems: 'center', gap: 12, padding: 14,
                    cursor: 'pointer',
                  }}>
                    <span style={{
                      width: 34, height: 34, borderRadius: 8,
                      background: T.paper, border: `1px solid ${T.line}`,
                      display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                      fontSize: 13, color: T.c.act, fontWeight: 700,
                    }}>{f.icon}</span>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ fontSize: 11, color: T.muted, letterSpacing: 0.3 }}>{f.l}</div>
                      <div style={{ fontSize: 13.5, color: T.fg, fontWeight: 600, marginTop: 1 }}>{f.v}</div>
                    </div>
                    <span style={{ fontSize: 11, color: T.c.act, fontWeight: 700 }}>edit</span>
                  </Tile>
                ))}
              </div>
            </div>

            {/* Step 4 — invite */}
            <div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 10 }}>
                <StepNum n="4" /><span style={{ fontWeight: 700, fontSize: 13, color: T.fg }}>Invite the crew</span>
              </div>
              <Tile style={{ padding: 12, display: 'flex', alignItems: 'center', gap: 12 }}>
                <Stack people={TEAM} size={28} ring={T.panel} />
                <div style={{ flex: 1, fontSize: 12.5, color: T.fg2 }}>
                  Nat, Lucas, Sam, Katie <span style={{ color: T.muted }}>· from Sprint 41</span>
                </div>
                <Btn kind="ghost">＋ add</Btn>
                <Btn kind="ghost">link</Btn>
              </Tile>
            </div>
          </div>

          {/* preview */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 18 }}>
            <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700 }}>LIVE PREVIEW</div>
            <div style={{
              borderRadius: 14, border: `1px solid ${T.line}`, overflow: 'hidden',
              boxShadow: T.s3,
            }}>
              <div className="sp-panel-grain" style={{
                padding: '10px 14px', background: T.panel,
                borderBottom: `1px solid ${T.line}`, fontSize: 11, color: T.muted,
                display: 'flex', justifyContent: 'space-between',
              }}>
                <span style={{ fontWeight: 600, color: T.fg }}>Sprint 42</span>
                <span>4 cols · 3 votes</span>
              </div>
              <div className="sp-paper" style={{ padding: 12 }}>
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 6 }}>
                  {['mood','well','wrong','act'].map((k) => (
                    <div key={k} style={{ display: 'flex', flexDirection: 'column', gap: 5 }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                        <span style={{ width: 5, height: 5, borderRadius: 3, background: T.c[k] }} />
                        <span style={{ fontSize: 8, color: T.muted, fontWeight: 700 }}>{k.toUpperCase()}</span>
                      </div>
                      <div style={{ background: T.c[k], borderRadius: 4, height: 22, opacity: 0.95, boxShadow: 'inset 0 -2px 4px rgba(0,0,0,0.1)' }} />
                      {k === 'well' && <div style={{ background: T.c[k], borderRadius: 4, height: 14, opacity: 0.75 }} />}
                    </div>
                  ))}
                </div>
              </div>
            </div>

            <Tile style={{ background: `${T.c.act}10`, border: `1px solid ${T.c.act}66` }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <span style={{ fontSize: 14 }}>💡</span>
                <span style={{ fontSize: 10.5, color: T.c.act, letterSpacing: 1, fontWeight: 800 }}>HOW IT WORKS</span>
              </div>
              <div style={{ marginTop: 8, fontSize: 12.5, color: T.fg, lineHeight: 1.55 }}>
                Board opens in <b>writing</b> mode. Drafts stay private until everyone marks ready. Anyone with the link can pin a card up.
              </div>
            </Tile>

            <Tile>
              <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1, fontWeight: 700 }}>OPENS WITH</div>
              <div style={{ marginTop: 8, display: 'flex', flexDirection: 'column', gap: 6, fontSize: 12.5, color: T.fg2 }}>
                <Line>your AI deck pre-filled from last week</Line>
                <Line>memory rail with open actions</Line>
                <Line>presence (4 ready)</Line>
              </div>
            </Tile>
          </div>
        </div>
      </Frame>
    </DCArtboard>
  );
};

const StepNum = ({ n }) => (
  <span style={{
    width: 22, height: 22, borderRadius: 11,
    background: T.fg, color: T.paper, fontSize: 11, fontWeight: 800,
    display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
    boxShadow: T.s1, flex: '0 0 auto',
  }}>{n}</span>
);

const Line = ({ children }) => (
  <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
    <span style={{ width: 12, height: 12, borderRadius: 6, background: T.c.well, color: '#fff', fontSize: 9, display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}>✓</span>
    <span>{children}</span>
  </span>
);

// ─── ③ WRITING ─────────────────────────────────────────
window.real_Writing = function () {
  const cols = [
    { k: 'mood',       c: T.c.mood,  count: 2 },
    { k: 'went well',  c: T.c.well,  count: 3 },
    { k: 'went wrong', c: T.c.wrong, count: 2 },
    { k: 'actions',    c: T.c.act,   count: '—', locked: true },
  ];
  return (
    <DCArtboard id="real-writing" label="③ writing" width={1240} height={780}>
      <Frame>
        <TopBar board="Sprint 42 · platform" phase="writing · 2 of 4 ready"
          presence={<Stack people={TEAM} size={26} ring={T.panel} />}
          right={<>
            <Pill tone="soft" accent={T.c.mood}><span className="sp-live-dot" style={{ width: 6, height: 6, background: T.c.mood }} />writing</Pill>
            <Btn kind="primary">i'm ready</Btn>
            <Btn kind="dashed">reveal →</Btn>
          </>}
        />
        <div style={{ flex: 1, padding: '20px 24px 150px', display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 18, position: 'relative', minHeight: 0 }}>
          {cols.map((col) => (
            <div key={col.k} style={{ display: 'flex', flexDirection: 'column', minHeight: 0 }}>
              <ColHeader name={col.k} count={col.count} color={col.c}
                sub={col.k === 'mood' ? '· one per person' : col.k === 'actions' ? '· opens after vote' : ''} />
              {!col.locked && (
                <ColAddCard accent={col.c} active={col.k === 'went well'}
                  hint={`add ${col.k} card`} />
              )}
              <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                {col.k === 'mood' && <>
                  <Card accent={col.c}>
                    <GifTile h={68} kind="a" />
                    <div style={{ marginTop: 8 }}>tired but alive</div>
                    <CardFooter author="na" color="#cf4f4f" tag="mood" />
                  </Card>
                  <HiddenDraft accent={col.c} />
                </>}
                {col.k === 'went well' && <>
                  <Card accent={col.c}>
                    deploy gate landed ahead of plan — months of work
                    <CardFooter author="na" color="#cf4f4f" tag="shipped" />
                  </Card>
                  <HiddenDraft accent={col.c} />
                  <Card accent={col.c}>
                    paired with @nat on the migrator. learned more than the last 3 sprints combined.
                    <CardFooter author="kt" color="#2f9469" tag="pairing" />
                  </Card>
                </>}
                {col.k === 'went wrong' && <>
                  <Card accent={col.c}>
                    <GifTile h={70} kind="b" />
                    <div style={{ marginTop: 8 }}>e2e flake. again. friday afternoon.</div>
                    <CardFooter author="na" color="#cf4f4f" tag="flake" />
                  </Card>
                  <HiddenDraft accent={col.c} />
                </>}
                {col.k === 'actions' && (
                  <div style={{
                    padding: 16, borderRadius: 10,
                    border: `1.5px dashed ${T.line2}`, background: `${T.c.act}06`,
                    color: T.muted, fontSize: 12, textAlign: 'center', lineHeight: 1.5,
                    display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8,
                  }}>
                    <span style={{ fontSize: 20, color: T.c.act, fontWeight: 700 }}>★</span>
                    <span>fills with top-voted<br/>after voting closes</span>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>

        {/* Floating deck */}
        <div style={{
          position: 'absolute', left: 20, right: 20, bottom: 18,
          background: 'rgba(251, 243, 223, 0.94)',
          backdropFilter: 'blur(12px)',
          border: `1px solid ${T.line}`, borderRadius: 14, padding: '12px 16px',
          boxShadow: T.s3,
          display: 'flex', flexDirection: 'column', gap: 10,
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <span style={{
              fontSize: 10, padding: '3px 8px', borderRadius: 99,
              background: T.c.act, color: '#fff', fontWeight: 800, letterSpacing: 0.5,
              display: 'inline-flex', alignItems: 'center', gap: 5,
            }}>✦ AI · YOUR DECK</span>
            <span style={{ fontFamily: T.hand, fontSize: 24, color: T.fg, lineHeight: 1 }}>spill suggests</span>
            <span style={{ fontSize: 11, color: T.muted }}>7 cards · pulled from this week's sessions</span>
            <div style={{ flex: 1 }} />
            <Pill tone="neutral">gifs · 12</Pill>
            <Pill tone="neutral">stickers</Pill>
            <span style={{ fontSize: 11, color: T.muted, fontStyle: 'italic' }}>↑ drag a card into any column</span>
            <Btn kind="ghost" style={{ padding: '4px 8px' }}>– minimize</Btn>
          </div>
          <div style={{ display: 'flex', gap: 10, overflow: 'hidden' }}>
            <DeckCard label="oncall friday wiped me out" source="incident · 5/17" sentiment="wrong" />
            <DeckCard label="shipped deploy gate ahead of plan" source="jira · 5/15" sentiment="well" />
            <DeckCard gif kind="c" source="giphy · top of mind" />
            <DeckCard label="paired w/ nat on migrator — learned a lot" source="sessions · 5/18" sentiment="well" />
            <DeckCard gif kind="d" source="giphy · victory" />
            <DeckCard label="onboarding doc still empty" source="slack · #plat" sentiment="wrong" />
            <DeckCard label="3 hrs lost to flake" source="sessions · w/o 5/13" sentiment="wrong" />
          </div>
        </div>
      </Frame>
    </DCArtboard>
  );
};

const CardFooter = ({ author, color, tag, votes }) => (
  <div style={{
    marginTop: 10, paddingTop: 8,
    borderTop: '1px solid rgba(255,255,255,0.18)',
    display: 'flex', alignItems: 'center', gap: 6, fontSize: 11,
  }}>
    <Avatar k={author} color={color} size={18} ring="rgba(255,255,255,0.6)" />
    {tag && <span style={{
      fontSize: 9.5, padding: '2px 7px', borderRadius: 99,
      background: 'rgba(255,255,255,0.18)', color: '#fff',
      fontWeight: 600, letterSpacing: 0.3,
    }}>#{tag}</span>}
    <div style={{ flex: 1 }} />
    {votes !== undefined && (
      <span style={{
        fontSize: 11, padding: '2px 8px', borderRadius: 99,
        background: '#fff', color: '#000', fontWeight: 700,
        display: 'inline-flex', alignItems: 'center', gap: 4,
      }}>
        <span style={{ fontSize: 8 }}>●●●</span> {votes}
      </span>
    )}
  </div>
);

const DeckCard = ({ label, source, sentiment, gif, kind = 'a' }) => {
  const accent = sentiment ? T.c[sentiment === 'well' ? 'well' : sentiment === 'wrong' ? 'wrong' : 'mood'] : T.muted;
  return (
    <div style={{
      flex: '0 0 auto', width: 195, padding: 10, borderRadius: 10,
      background: T.paper, border: `1px solid ${T.line}`,
      display: 'flex', flexDirection: 'column', gap: 8,
      cursor: 'grab', boxShadow: T.s1,
      position: 'relative',
    }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 5, fontSize: 9, color: T.muted, fontWeight: 700, letterSpacing: 0.4 }}>
        <span style={{
          padding: '2px 6px', borderRadius: 99,
          background: gif ? T.fg : `${accent}1c`, color: gif ? T.paper : accent,
          border: gif ? 'none' : `1px solid ${accent}55`, letterSpacing: 0.5,
        }}>{gif ? 'GIF' : 'AI'}</span>
        {sentiment && <span style={{ width: 5, height: 5, borderRadius: 3, background: accent }} />}
        <div style={{ flex: 1 }} />
        <span style={{ color: T.muted, fontSize: 10 }}>⋮⋮</span>
      </div>
      {gif ? <GifTile h={66} kind={kind} label="GIF" /> : (
        <div style={{ fontSize: 12.5, color: T.fg, lineHeight: 1.35, fontWeight: 500 }}>{label}</div>
      )}
      {source && <div style={{ fontSize: 10, color: T.muted, fontStyle: 'italic' }}>{source}</div>}
    </div>
  );
};

// ─── ④ CLUSTER + VOTING ───────────────────────────────
const Cluster = ({ accent, title, cards, votes, suggested, votingLeft }) => (
  <div style={{
    position: 'relative', borderRadius: 12, padding: '12px 12px 12px',
    background: `${accent}0e`,
    border: `1.5px ${suggested ? 'dashed' : 'solid'} ${accent}aa`,
    marginBottom: 12, boxShadow: T.s1,
  }}>
    <div style={{
      position: 'absolute', top: -10, left: 12,
      padding: '3px 9px', borderRadius: 99,
      background: T.paper, color: accent,
      fontSize: 9.5, fontWeight: 800, letterSpacing: 1,
      border: `1px solid ${accent}80`, display: 'inline-flex', alignItems: 'center', gap: 5,
    }}>
      <span>{suggested ? '◇' : '◆'}</span>
      {suggested ? 'SUGGESTED' : 'CLUSTER'} · {title}
    </div>
    {votes !== undefined ? (
      <span style={{
        position: 'absolute', top: -10, right: 10,
        padding: '3px 10px', borderRadius: 99,
        background: accent, color: '#fff',
        fontSize: 10, fontWeight: 800, letterSpacing: 0.4,
        boxShadow: `0 2px 4px ${accent}55`,
        display: 'inline-flex', alignItems: 'center', gap: 5,
      }}>★ {votes} {votes === 1 ? 'vote' : 'votes'}</span>
    ) : (
      <Pill tone="ghost" style={{
        position: 'absolute', top: -10, right: 10,
        background: T.paper, color: T.muted,
      }}>vote the cluster</Pill>
    )}
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8, paddingTop: 6 }}>
      {cards.map((c, i) => (
        <div key={i} style={{
          background: accent, color: '#fff',
          borderRadius: 8, padding: '8px 12px',
          fontFamily: T.font, fontSize: 12.5, lineHeight: 1.35, fontWeight: 500,
          marginLeft: i === 1 ? 10 : (i === 2 ? 5 : 0),
          boxShadow: `0 1px 0 rgba(0,0,0,0.08), inset 0 1px 0 rgba(255,255,255,0.18)`,
          display: 'flex', alignItems: 'center', gap: 8,
        }}>
          <Avatar k={c.who} color={c.wc} size={18} ring="rgba(255,255,255,0.5)" />
          <span style={{ flex: 1 }}>{c.t}</span>
        </div>
      ))}
    </div>
    {suggested && (
      <div style={{ marginTop: 10, display: 'flex', gap: 6, alignItems: 'center' }}>
        <Btn kind="primary" accent={T.c.well}>✓ accept</Btn>
        <Btn kind="ghost">edit name</Btn>
        <Btn kind="ghost">split</Btn>
        <div style={{ flex: 1 }} />
        <span style={{ fontSize: 10.5, color: T.muted, fontStyle: 'italic' }}>spill drafted this</span>
      </div>
    )}
  </div>
);

window.real_Cluster = function () {
  return (
    <DCArtboard id="real-cluster" label="④ cluster-fy · suggested" width={1240} height={760}>
      <Frame>
        <TopBar board="Sprint 42 · platform" phase="cluster-fy · review proposed groupings"
          presence={<Stack people={TEAM.map(t => ({ ...t, status: 'voting' }))} size={26} ring={T.panel} />}
          right={<>
            <Pill tone="soft" accent={T.c.act}><span className="sp-live-dot" style={{ background: T.c.act, width: 6, height: 6 }} />cluster-fy</Pill>
            <Pill tone="neutral">3 suggested · 1 accepted</Pill>
            <Btn kind="primary" accent={T.c.well}>accept all</Btn>
            <Btn kind="dashed">skip →</Btn>
          </>}
        />
        <div style={{ flex: 1, padding: '20px 24px', display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 18, overflow: 'hidden' }}>
          <div>
            <ColHeader name="mood" count="3" color={T.c.mood} />
            <Card accent={T.c.mood} style={{ marginBottom: 10 }}>
              tired but alive<CardFooter author="na" color="#cf4f4f" tag="mood" />
            </Card>
            <Card accent={T.c.mood} style={{ marginBottom: 10 }}>
              caffeinated chaos<CardFooter author="lu" color="#cf8a3f" tag="mood" />
            </Card>
            <Card accent={T.c.mood}>
              wired but ok<CardFooter author="sa" color="#8757b6" tag="mood" />
            </Card>
          </div>

          <div>
            <ColHeader name="went well" count="4 · 1 cluster" color={T.c.well} />
            <Cluster accent={T.c.well} title="DEPLOYS" suggested
              cards={[
                { t: 'deploy gate landed ahead', who: 'na', wc: '#cf4f4f' },
                { t: 'shipped 3 days early', who: 'kt', wc: '#2f9469' },
              ]}
            />
            <Card accent={T.c.well} style={{ marginBottom: 10 }}>
              paired w/ nat on migrator — learned a lot
              <CardFooter author="kt" color="#2f9469" tag="pairing" />
            </Card>
            <Card accent={T.c.well}>
              customer demo went well
              <CardFooter author="sa" color="#8757b6" tag="demo" />
            </Card>
          </div>

          <div>
            <ColHeader name="went wrong" count="5 · 1 cluster" color={T.c.wrong} />
            <Cluster accent={T.c.wrong} title="THE FLAKE" suggested
              cards={[
                { t: 'e2e flake AGAIN friday', who: 'na', wc: '#cf4f4f' },
                { t: 'stage timeouts cost me 30m', who: 'lu', wc: '#cf8a3f' },
                { t: 'flake suite still red', who: 'sa', wc: '#8757b6' },
              ]}
            />
            <Card accent={T.c.wrong}>
              onboarding doc still empty
              <CardFooter author="na" color="#cf4f4f" tag="docs" />
            </Card>
          </div>

          <div>
            <ColHeader name="actions" count="—" color={T.c.act} />
            <div style={{
              padding: 18, borderRadius: 12,
              background: `${T.c.act}0a`,
              border: `1.5px dashed ${T.c.act}55`,
              color: T.muted, fontSize: 12, lineHeight: 1.5,
              display: 'flex', flexDirection: 'column', gap: 10,
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6, color: T.c.act, fontWeight: 700, letterSpacing: 1, fontSize: 10 }}>
                <span>◆</span> ONE-OFF
              </div>
              <div style={{ color: T.fg, fontSize: 13 }}>
                Clusters are <b>one-time</b>. Once voting starts they freeze. You can split or undo from card menu.
              </div>
              <div style={{ marginTop: 4, paddingTop: 10, borderTop: `1px dashed ${T.line}`, color: T.muted }}>
                <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1, fontWeight: 700 }}>UP NEXT</div>
                <div style={{ marginTop: 6, fontSize: 12, color: T.fg2 }}>
                  → vote round (3 votes each)<br/>
                  → top 3 move to action discussion
                </div>
              </div>
            </div>
          </div>
        </div>
      </Frame>
    </DCArtboard>
  );
};

window.real_Voting = function () {
  return (
    <DCArtboard id="real-voting" label="⑤ voting" width={1240} height={760}>
      <Frame>
        <TopBar board="Sprint 42 · platform" phase="voting · 2 votes left"
          presence={<Stack people={TEAM.map(t => ({ ...t, status: 'voting' }))} size={26} ring={T.panel} />}
          right={<>
            <Pill tone="soft" accent={T.c.mood}><span className="sp-live-dot" style={{ background: T.c.mood, width: 6, height: 6 }} />voting</Pill>
            <Pill tone="solid" accent={T.c.wrong}>● ● ○  2 left</Pill>
            <Btn kind="primary">i'm done</Btn>
            <Btn kind="dashed">actions →</Btn>
          </>}
        />
        <div style={{ flex: 1, padding: '20px 24px', display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 18, overflow: 'hidden' }}>
          <div>
            <ColHeader name="mood" count="3" color={T.c.mood} />
            <Card accent={T.c.mood} style={{ marginBottom: 10 }}>tired but alive<CardFooter author="na" color="#cf4f4f" tag="mood" /></Card>
            <Card accent={T.c.mood} style={{ marginBottom: 10 }}>caffeinated chaos<CardFooter author="lu" color="#cf8a3f" tag="mood" /></Card>
            <Card accent={T.c.mood}>wired but ok<CardFooter author="sa" color="#8757b6" tag="mood" /></Card>
          </div>

          <div>
            <ColHeader name="went well" count="4 · 1 cluster" color={T.c.well} />
            <Cluster accent={T.c.well} title="DEPLOYS" votes={2}
              cards={[
                { t: 'deploy gate landed ahead', who: 'na', wc: '#cf4f4f' },
                { t: 'shipped 3 days early', who: 'kt', wc: '#2f9469' },
              ]}
            />
            <Card accent={T.c.well} style={{ marginBottom: 10 }}>
              paired w/ nat on migrator — learned a lot
              <CardFooter author="kt" color="#2f9469" tag="pairing" votes={1} />
            </Card>
            <Card accent={T.c.well}>
              customer demo went well
              <CardFooter author="sa" color="#8757b6" tag="demo" />
            </Card>
          </div>

          <div>
            <ColHeader name="went wrong" count="5 · 1 cluster" color={T.c.wrong} />
            <Cluster accent={T.c.wrong} title="THE FLAKE" votes={4}
              cards={[
                { t: 'e2e flake AGAIN friday', who: 'na', wc: '#cf4f4f' },
                { t: 'stage timeouts cost me 30m', who: 'lu', wc: '#cf8a3f' },
                { t: 'flake suite still red', who: 'sa', wc: '#8757b6' },
              ]}
            />
            <Card accent={T.c.wrong}>
              onboarding doc still empty
              <CardFooter author="na" color="#cf4f4f" tag="docs" votes={1} />
            </Card>
          </div>

          <div>
            <ColHeader name="leaderboard" count="live" color={T.c.act} />
            <Tile style={{ padding: 16 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <span className="sp-live-dot" style={{ width: 6, height: 6 }} />
                <span style={{ fontSize: 10, color: T.muted, letterSpacing: 1.2, fontWeight: 800 }}>LIVE STANDINGS</span>
              </div>
              <div style={{ marginTop: 12, display: 'flex', flexDirection: 'column', gap: 14 }}>
                {[
                  { t: 'THE FLAKE', sub: 'cluster · 3 cards', v: 4, max: 4, c: T.c.wrong },
                  { t: 'DEPLOYS',   sub: 'cluster · 2 cards', v: 2, max: 4, c: T.c.well },
                  { t: 'paired w/ nat', sub: 'kt', v: 1, max: 4, c: T.c.well },
                  { t: 'onboarding doc', sub: 'na', v: 1, max: 4, c: T.c.wrong },
                ].map((r, i) => (
                  <div key={r.t}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 12, marginBottom: 5 }}>
                      <span style={{ color: T.fg, fontWeight: 600 }}>
                        {i < 2 && <span style={{ color: r.c, marginRight: 5 }}>#{i + 1}</span>}
                        {r.t}
                        <span style={{ color: T.muted, fontWeight: 400, marginLeft: 6, fontSize: 11 }}>· {r.sub}</span>
                      </span>
                      <span style={{ color: r.c, fontWeight: 700 }}>{r.v}</span>
                    </div>
                    <div style={{ height: 6, background: T.paper2, borderRadius: 3 }}>
                      <div style={{
                        width: `${(r.v / 4) * 100}%`, height: '100%',
                        background: `linear-gradient(90deg, ${shade(r.c, 4)}, ${r.c})`,
                        borderRadius: 3, boxShadow: `0 0 8px ${r.c}55`,
                      }} />
                    </div>
                  </div>
                ))}
              </div>
              <div style={{ marginTop: 14, paddingTop: 12, borderTop: `1px dashed ${T.line}`, fontSize: 11, color: T.muted, lineHeight: 1.4 }}>
                top 3 move to <b style={{ color: T.fg2 }}>action discussion</b> when round closes
              </div>
            </Tile>

            <Tile style={{ marginTop: 14, padding: 14 }}>
              <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700 }}>YOUR VOTES</div>
              <div style={{ marginTop: 8, display: 'flex', gap: 6, justifyContent: 'space-between' }}>
                {[1, 2, 3].map((n) => (
                  <span key={n} style={{
                    flex: 1, padding: '8px 0', borderRadius: 8,
                    background: n === 1 ? T.c.wrong : T.paper2,
                    color: n === 1 ? '#fff' : T.muted,
                    border: `1px solid ${n === 1 ? shade(T.c.wrong, -10) : T.line}`,
                    textAlign: 'center', fontSize: 11, fontWeight: 700,
                  }}>{n === 1 ? '✓ spent' : 'open'}</span>
                ))}
              </div>
              <div style={{ marginTop: 8, fontSize: 11, color: T.muted, fontStyle: 'italic' }}>
                spent: THE FLAKE
              </div>
            </Tile>
          </div>
        </div>
      </Frame>
    </DCArtboard>
  );
};

// ─── ⑥ ACTION DISCUSSION ────────────────────────────
window.real_Action = function () {
  return (
    <DCArtboard id="real-action" label="⑥ action discussion" width={1240} height={760}>
      <Frame>
        <TopBar board="Sprint 42 · platform" phase="action discussion · #1 of 3"
          presence={<Stack people={TEAM.map(t => ({ ...t, status: 'ready' }))} size={26} ring={T.panel} />}
          right={<>
            <Pill tone="soft" accent={T.c.act}><span className="sp-live-dot" style={{ background: T.c.act, width: 6, height: 6 }} />action</Pill>
            <Btn kind="ghost">← prev</Btn>
            <Btn kind="ghost">next →</Btn>
            <Btn kind="dashed">wrap retro</Btn>
          </>}
        />
        <div style={{ flex: 1, padding: '22px 28px', display: 'grid', gridTemplateColumns: '230px 1fr 360px', gap: 22, overflow: 'hidden' }}>
          {/* Agenda */}
          <div style={{ minWidth: 0 }}>
            <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700, marginBottom: 10 }}>AGENDA · 3</div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {[
                { n: 1, t: 'THE FLAKE',     v: 4, active: true,  c: T.c.wrong, status: 'in progress' },
                { n: 2, t: 'DEPLOYS',       v: 2, c: T.c.well,   status: 'queued' },
                { n: 3, t: 'onboarding doc', v: 1, c: T.c.wrong, status: 'queued' },
              ].map((a) => (
                <div key={a.n} style={{
                  padding: 12, borderRadius: 10,
                  background: a.active ? `${a.c}12` : T.panel,
                  border: `${a.active ? 2 : 1}px solid ${a.active ? a.c : T.line}`,
                  boxShadow: a.active ? T.s2 : T.s1, cursor: 'pointer',
                  position: 'relative',
                }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    <span style={{
                      width: 22, height: 22, borderRadius: 11,
                      background: a.active ? a.c : T.paper2, color: a.active ? '#fff' : T.muted,
                      border: `1px solid ${a.active ? shade(a.c, -10) : T.line}`,
                      fontSize: 11, fontWeight: 800,
                      display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                    }}>{a.n}</span>
                    <span style={{ fontSize: 12.5, color: T.fg, flex: 1, fontWeight: 700, letterSpacing: -0.1 }}>{a.t}</span>
                    <span style={{ fontSize: 11, color: T.muted, fontWeight: 600 }}>{a.v}★</span>
                  </div>
                  <div style={{ marginTop: 6, fontSize: 10, color: a.active ? a.c : T.muted, letterSpacing: 0.4, textTransform: 'uppercase', fontWeight: 700 }}>{a.status}</div>
                </div>
              ))}
            </div>

            <div style={{ marginTop: 16, padding: 12, background: T.panel, borderRadius: 10, border: `1px dashed ${T.line}`, fontSize: 11, color: T.muted, lineHeight: 1.5 }}>
              <span style={{ color: T.fg2, fontWeight: 600 }}>tie-breakers?</span> toggle items in/out inline. host calls it.
            </div>

            <div style={{ marginTop: 16, fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700 }}>TIMING</div>
            <div style={{ marginTop: 8, padding: 12, borderRadius: 10, background: T.panelHi, border: `1px solid ${T.line}` }}>
              <div style={{ fontSize: 11, color: T.muted }}>elapsed</div>
              <div style={{ fontSize: 22, fontWeight: 800, color: T.fg, letterSpacing: -0.5, fontVariantNumeric: 'tabular-nums' }}>14:32</div>
              <div style={{ marginTop: 6, height: 4, background: T.paper2, borderRadius: 2 }}>
                <div style={{ width: '48%', height: '100%', background: T.c.act, borderRadius: 2 }} />
              </div>
              <div style={{ marginTop: 6, fontSize: 10.5, color: T.muted }}>~16 min left at this pace</div>
            </div>
          </div>

          {/* Focus */}
          <div style={{ minWidth: 0 }}>
            <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700, marginBottom: 10 }}>UNDER DISCUSSION</div>
            <div style={{
              padding: 22, borderRadius: 16,
              background: `linear-gradient(180deg, ${shade(T.c.wrong, 4)} 0%, ${T.c.wrong} 70%, ${shade(T.c.wrong, -8)} 100%)`,
              color: '#fff', position: 'relative',
              boxShadow: `${T.s3}, inset 0 1px 0 rgba(255,255,255,0.18)`,
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 10.5, opacity: 0.95, letterSpacing: 1, fontWeight: 800, textTransform: 'uppercase' }}>
                <span>◆ CLUSTER</span><span>·</span><span>3 CARDS</span><span>·</span><span>★ 4 VOTES</span>
              </div>
              <div style={{ fontSize: 38, fontWeight: 800, marginTop: 6, letterSpacing: -1, lineHeight: 1 }}>THE FLAKE</div>
              <div style={{ fontSize: 13, opacity: 0.85, marginTop: 6, fontStyle: 'italic' }}>third sprint in a row — time to actually do something</div>

              <div style={{ marginTop: 16, display: 'flex', flexDirection: 'column', gap: 8 }}>
                {[
                  { t: 'e2e flake AGAIN friday — lost the afternoon', who: 'na', wc: '#cf4f4f' },
                  { t: 'stage timeouts cost me 30m', who: 'lu', wc: '#cf8a3f' },
                  { t: 'flake suite still red. it never went green.', who: 'sa', wc: '#8757b6' },
                ].map((c, i) => (
                  <div key={i} style={{
                    padding: '10px 12px', background: 'rgba(255,255,255,0.13)',
                    borderRadius: 8, fontSize: 13, lineHeight: 1.4,
                    display: 'flex', alignItems: 'center', gap: 10,
                    border: '1px solid rgba(255,255,255,0.1)',
                  }}>
                    <Avatar k={c.who} color={c.wc} size={22} ring="rgba(255,255,255,0.5)" />
                    <span style={{ flex: 1 }}>{c.t}</span>
                  </div>
                ))}
              </div>
            </div>

            <div style={{ marginTop: 14, fontSize: 11, color: T.muted, fontStyle: 'italic', textAlign: 'center' }}>
              originals stay readable · who said what is preserved
            </div>
          </div>

          {/* Proposed actions */}
          <div style={{ minWidth: 0 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginBottom: 10 }}>
              <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700 }}>PROPOSED ACTIONS</div>
              <span style={{ fontSize: 11, color: T.c.act, fontWeight: 700, cursor: 'pointer' }}>↻ regenerate</span>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              <Tile style={{ padding: 14, border: `1.5px solid ${T.c.well}88`, background: `${T.c.well}0c` }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 6 }}>
                  <span style={{ fontSize: 9.5, color: T.c.well, fontWeight: 800, letterSpacing: 0.5, padding: '2px 7px', borderRadius: 99, background: `${T.c.well}1c`, border: `1px solid ${T.c.well}55` }}>✦ AI</span>
                </div>
                <div style={{ fontSize: 14, color: T.fg, fontWeight: 700, lineHeight: 1.35 }}>quarantine flake suite, kill next sprint</div>
                <div style={{ marginTop: 12, display: 'flex', gap: 6, alignItems: 'center' }}>
                  <span style={{
                    display: 'inline-flex', alignItems: 'center', gap: 5,
                    padding: '4px 9px 4px 4px', borderRadius: 99,
                    background: T.panelHi, border: `1px solid ${T.line}`,
                    fontSize: 11, color: T.fg, fontWeight: 600,
                  }}>
                    <Avatar k="lu" color="#cf8a3f" size={18} ring={T.panelHi} />
                    lucas
                  </span>
                  <Pill tone="neutral">fri 5/30</Pill>
                  <div style={{ flex: 1 }} />
                  <Btn kind="primary" accent={T.c.well}>✓ confirm</Btn>
                </div>
              </Tile>

              <Tile style={{ padding: 14 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 6 }}>
                  <span style={{ fontSize: 9.5, color: T.muted, fontWeight: 700, letterSpacing: 0.5, padding: '2px 7px', borderRadius: 99, border: `1px solid ${T.line}` }}>HUMAN</span>
                </div>
                <div style={{ fontSize: 14, color: T.fg, fontWeight: 600, lineHeight: 1.35 }}>flake counter on deploy gate</div>
                <div style={{ marginTop: 12, display: 'flex', gap: 6 }}>
                  <Btn kind="dashed">＋ owner</Btn>
                  <Btn kind="dashed">＋ due</Btn>
                  <div style={{ flex: 1 }} />
                  <Btn kind="secondary">confirm</Btn>
                </div>
              </Tile>

              <div style={{
                padding: 12, borderRadius: 10,
                border: `1.5px dashed ${T.line2}`,
                fontSize: 13, color: T.muted, textAlign: 'center', cursor: 'pointer',
              }}>＋ add your own</div>
            </div>

            <div style={{ marginTop: 16, paddingTop: 14, borderTop: `1px dashed ${T.line}` }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700 }}>CONFIRMED</div>
                <Pill tone="soft" accent={T.c.well}>1 / 3</Pill>
              </div>
              <div style={{ marginTop: 8, fontSize: 12.5, color: T.fg, display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{
                  width: 16, height: 16, borderRadius: 4,
                  background: T.c.well, color: '#fff',
                  display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                  fontSize: 11, fontWeight: 800,
                }}>✓</span>
                <span style={{ flex: 1, fontWeight: 500 }}>quarantine flake suite</span>
                <span style={{ color: T.muted, fontSize: 11 }}>@lucas · 5/30</span>
              </div>
            </div>
          </div>
        </div>
      </Frame>
    </DCArtboard>
  );
};

// ─── ⑦ SUMMARY ────────────────────────────────────
window.real_Summary = function () {
  return (
    <DCArtboard id="real-summary" label="⑦ wrapped" width={1240} height={760}>
      <Frame>
        <TopBar board="Sprint 42 · platform" phase="completed · tue 5/21 · 38 minutes"
          right={<>
            <Pill tone="soft" accent={T.c.well}>✓ done</Pill>
            <Btn kind="secondary">share link</Btn>
            <Btn kind="secondary">slack</Btn>
            <Btn kind="primary">export</Btn>
          </>}
        />
        <div style={{ flex: 1, padding: '26px 32px', display: 'grid', gridTemplateColumns: '1fr 340px', gap: 32, overflow: 'hidden' }}>
          <div style={{ minWidth: 0 }}>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: 12 }}>
              <h1 style={{ margin: 0, fontFamily: T.font, fontSize: 38, fontWeight: 800, letterSpacing: -1, color: T.fg }}>
                That's a wrap.
              </h1>
              <span style={{ fontFamily: T.hand, fontSize: 24, color: T.muted, transform: 'rotate(-2deg)' }}>nice work, team.</span>
            </div>

            {/* Mood */}
            <div style={{ marginTop: 18, display: 'flex', alignItems: 'center', gap: 18 }}>
              <div style={{
                width: 100, height: 100, borderRadius: 50,
                background: `radial-gradient(circle at 35% 30%, ${shade(T.c.well, 8)}, ${T.c.well} 70%)`,
                border: `2px solid ${shade(T.c.well, -12)}`,
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                fontFamily: T.font, fontSize: 22, fontWeight: 800, color: '#fff',
                boxShadow: `${T.s3}, inset 0 2px 0 rgba(255,255,255,0.25)`,
                letterSpacing: -0.4, flex: '0 0 auto',
              }}>steady</div>
              <div>
                <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700 }}>TEAM MOOD</div>
                <div style={{ fontSize: 24, fontWeight: 800, color: T.fg, marginTop: 2, letterSpacing: -0.5 }}>Steady.</div>
                <div style={{ fontSize: 13.5, color: T.fg2, marginTop: 4, maxWidth: 500, lineHeight: 1.5 }}>
                  Shipping felt good. Flakiness is still weighing — third sprint with the same e2e pain. Action confirmed.
                </div>
              </div>
            </div>

            <div style={{ marginTop: 22, display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14 }}>
              <Tile style={{ padding: 16 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ width: 8, height: 8, borderRadius: 4, background: T.c.well }} />
                  <div style={{ fontSize: 10.5, color: T.c.well, letterSpacing: 1.2, fontWeight: 800 }}>WINS · 3</div>
                </div>
                <ul style={{ paddingLeft: 16, margin: '10px 0 0', color: T.fg, fontSize: 13.5, lineHeight: 1.6 }}>
                  <li>deploy gate, finally</li>
                  <li>customer demo landed early</li>
                  <li>paired migrator went smooth</li>
                </ul>
              </Tile>
              <Tile style={{ padding: 16 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ width: 8, height: 8, borderRadius: 4, background: T.c.wrong }} />
                  <div style={{ fontSize: 10.5, color: T.c.wrong, letterSpacing: 1.2, fontWeight: 800 }}>PAINS · 2</div>
                </div>
                <ul style={{ paddingLeft: 16, margin: '10px 0 0', color: T.fg, fontSize: 13.5, lineHeight: 1.6 }}>
                  <li>e2e flake <span style={{ color: T.c.wrong, fontWeight: 700 }}>(3rd retro)</span></li>
                  <li>onboarding doc still missing</li>
                </ul>
              </Tile>
            </div>

            {/* Actions */}
            <div style={{ marginTop: 20 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span style={{ width: 8, height: 8, borderRadius: 4, background: T.c.act }} />
                <div style={{ fontSize: 10.5, color: T.c.act, letterSpacing: 1.2, fontWeight: 800 }}>ACTIONS COMMITTED · 3</div>
              </div>
              <div style={{ marginTop: 10, display: 'flex', flexDirection: 'column', gap: 8 }}>
                {[
                  { t: 'quarantine flake suite, kill next sprint', who: 'lucas', wc: '#cf8a3f', when: 'fri 5/30' },
                  { t: 'flake counter on deploy gate', who: 'sam', wc: '#8757b6', when: 'tue 6/4' },
                  { t: 'own the onboarding doc', who: 'nat', wc: '#cf4f4f', when: 'mon 5/27' },
                ].map((a, i) => (
                  <Tile key={i} style={{ padding: '10px 14px', display: 'flex', alignItems: 'center', gap: 10 }}>
                    <span style={{
                      width: 22, height: 22, borderRadius: 6,
                      background: T.c.act, color: '#fff',
                      display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                      fontSize: 12, fontWeight: 800, flex: '0 0 auto',
                    }}>✓</span>
                    <span style={{ flex: 1, fontSize: 13.5, color: T.fg, fontWeight: 500 }}>{a.t}</span>
                    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
                      <Avatar k={a.who.slice(0,2)} color={a.wc} size={20} />
                      <span style={{ fontSize: 12, color: T.fg2, fontWeight: 600 }}>{a.who}</span>
                    </span>
                    <Pill tone="neutral">{a.when}</Pill>
                  </Tile>
                ))}
              </div>
            </div>

            <Tile style={{ marginTop: 18, padding: 14, display: 'flex', alignItems: 'center', gap: 14, background: T.panelHi }}>
              <span style={{
                width: 36, height: 36, borderRadius: 18,
                background: T.fg, color: T.paper,
                display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
                fontSize: 18, fontWeight: 800,
              }}>✈︎</span>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: 13.5, color: T.fg, fontWeight: 600 }}>
                  Delivered to <b>#team-platform</b>
                </div>
                <div style={{ fontSize: 11.5, color: T.muted }}>3 jira tickets created · summary saved</div>
              </div>
              <Pill tone="soft" accent={T.c.well}>✓ sent</Pill>
            </Tile>
          </div>

          {/* Right rail */}
          <div style={{ borderLeft: `1px solid ${T.line}`, paddingLeft: 22, display: 'flex', flexDirection: 'column', gap: 18 }}>
            <div>
              <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700 }}>WHO PARTICIPATED</div>
              <Tile style={{ marginTop: 8, padding: 12 }}>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
                  {[
                    { k: 'na', name: 'Nat',   role: 'host',    color: '#cf4f4f', cards: 4, votes: 3 },
                    { k: 'lu', name: 'Lucas', role: 'on-call', color: '#cf8a3f', cards: 3, votes: 3 },
                    { k: 'sa', name: 'Sam',   role: '',        color: '#8757b6', cards: 5, votes: 3 },
                    { k: 'kt', name: 'Katie', role: '',        color: '#2f9469', cards: 2, votes: 2 },
                  ].map((u) => (
                    <div key={u.k} style={{ display: 'flex', alignItems: 'center', gap: 10, fontSize: 12.5 }}>
                      <Avatar k={u.k} color={u.color} size={28} ring={T.panel} />
                      <div style={{ flex: 1 }}>
                        <div style={{ fontWeight: 700, color: T.fg }}>{u.name}
                          {u.role && <span style={{ fontSize: 10.5, color: T.muted, fontWeight: 500, marginLeft: 5 }}>· {u.role}</span>}
                        </div>
                        <div style={{ fontSize: 10.5, color: T.muted, marginTop: 1 }}>{u.cards} cards · {u.votes} votes</div>
                      </div>
                    </div>
                  ))}
                </div>
              </Tile>
            </div>

            <div>
              <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700 }}>STILL ON THE WALL</div>
              <Tile style={{
                marginTop: 8, padding: 14,
                background: `${T.c.wrong}10`, border: `1px solid ${T.c.wrong}66`,
              }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <span style={{ width: 7, height: 7, borderRadius: 4, background: T.c.wrong }} />
                  <span style={{ fontSize: 10, letterSpacing: 1, color: T.c.wrong, fontWeight: 800 }}>RECURRING</span>
                </div>
                <div style={{ fontSize: 16, fontWeight: 700, color: T.fg, marginTop: 4 }}>"flaky tests"</div>
                <div style={{ fontSize: 11.5, color: T.muted, marginTop: 2 }}>3 retros · 2 actions open</div>
                <div style={{ marginTop: 10, display: 'flex', gap: 5 }}>
                  {['s40','s41','s42'].map((s, i) => (
                    <span key={s} style={{
                      fontSize: 10, padding: '3px 8px', borderRadius: 99,
                      background: i === 2 ? T.c.wrong : `${T.c.wrong}33`,
                      color: i === 2 ? '#fff' : T.c.wrong, fontWeight: 700,
                    }}>{s}</span>
                  ))}
                </div>
              </Tile>
            </div>

            <div style={{ paddingTop: 14, borderTop: `1px dashed ${T.line}` }}>
              <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1.2, fontWeight: 700 }}>NEXT TIME · OPENS 6/4</div>
              <Tile style={{ marginTop: 8, padding: 14 }}>
                <div style={{ fontSize: 15, fontWeight: 700, color: T.fg }}>Sprint 43</div>
                <div style={{ fontSize: 11, color: T.muted, marginTop: 2 }}>auto-scheduled · same crew</div>
                <div style={{ marginTop: 10 }}>
                  <Btn kind="dashed">tweak settings</Btn>
                </div>
              </Tile>
            </div>
          </div>
        </div>
      </Frame>
    </DCArtboard>
  );
};

// ─── DESIGN TOKENS / SYSTEM REFERENCE ─────────────
window.real_DesignSystem = function () {
  return (
    <DCArtboard id="real-system" label="design tokens" width={1240} height={680}>
      <Frame>
        <TopBar board="Design System" phase="Spill. · Daylight Cork · applied"
          right={<>
            <Btn kind="ghost">spec sheet</Btn>
            <Btn kind="secondary">tokens.json</Btn>
          </>}
        />
        <div style={{ flex: 1, padding: '24px 32px', display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 26, overflow: 'auto' }} className="sp-scroll">
          {/* Type */}
          <div>
            <Section label="TYPE" />
            <Tile>
              <div style={{ fontFamily: T.font, fontWeight: 800, fontSize: 32, letterSpacing: -0.8, color: T.fg }}>Display 32</div>
              <div style={{ fontFamily: T.font, fontWeight: 700, fontSize: 20, color: T.fg, marginTop: 6 }}>Heading 20</div>
              <div style={{ fontFamily: T.font, fontWeight: 600, fontSize: 14, color: T.fg2, marginTop: 6 }}>Subhead 14</div>
              <div style={{ fontFamily: T.font, fontWeight: 400, fontSize: 13.5, color: T.fg, marginTop: 6, lineHeight: 1.5 }}>Body 13.5 — the workhorse. Reads cleanly on cream paper.</div>
              <div style={{ fontFamily: T.font, fontSize: 11, color: T.muted, marginTop: 6, letterSpacing: 1.2, fontWeight: 700 }}>LABEL 11 / 1.2</div>
              <div style={{ marginTop: 12, paddingTop: 12, borderTop: `1px dashed ${T.line}` }}>
                <div style={{ fontFamily: T.hand, fontSize: 28, color: T.fg, transform: 'rotate(-1.5deg)' }}>handwritten · for warmth</div>
                <div style={{ fontSize: 10.5, color: T.muted, fontStyle: 'italic', marginTop: 6 }}>Caveat — one per screen, max.</div>
              </div>
            </Tile>
          </div>

          {/* Color */}
          <div>
            <Section label="COLOR" />
            <Tile>
              <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1, fontWeight: 700, marginBottom: 8 }}>SURFACE</div>
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 6 }}>
                {[
                  ['paper', T.paper],
                  ['panel', T.panel],
                  ['panelHi', T.panelHi],
                  ['line', T.line],
                ].map(([n, c]) => (
                  <div key={n}>
                    <div style={{ height: 38, background: c, borderRadius: 6, border: `1px solid ${T.line2}` }} />
                    <div style={{ fontSize: 9.5, color: T.muted, marginTop: 4, fontWeight: 600 }}>{n}</div>
                    <div style={{ fontSize: 8.5, color: T.muted, fontVariantNumeric: 'tabular-nums' }}>{c}</div>
                  </div>
                ))}
              </div>
              <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1, fontWeight: 700, marginTop: 14, marginBottom: 8 }}>COLUMN ACCENTS</div>
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 6 }}>
                {[
                  ['mood', T.c.mood],
                  ['well', T.c.well],
                  ['wrong', T.c.wrong],
                  ['act', T.c.act],
                ].map(([n, c]) => (
                  <div key={n}>
                    <div style={{ height: 38, background: c, borderRadius: 6, boxShadow: T.s1 }} />
                    <div style={{ fontSize: 9.5, color: T.muted, marginTop: 4, fontWeight: 600 }}>{n}</div>
                    <div style={{ fontSize: 8.5, color: T.muted, fontVariantNumeric: 'tabular-nums' }}>{c}</div>
                  </div>
                ))}
              </div>
              <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1, fontWeight: 700, marginTop: 14, marginBottom: 8 }}>USAGE</div>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 4, fontSize: 11.5, color: T.fg2 }}>
                <span>· paper = canvas bg (always)</span>
                <span>· panel = top bar, tiles, lifts off paper</span>
                <span>· accents = column membership only</span>
                <span>· tints at /1c (~11%) for soft fills</span>
              </div>
            </Tile>
          </div>

          {/* Components */}
          <div>
            <Section label="COMPONENTS" />
            <Tile style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
              <div>
                <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1, fontWeight: 700, marginBottom: 8 }}>BUTTONS</div>
                <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                  <Btn kind="primary">primary</Btn>
                  <Btn kind="secondary">secondary</Btn>
                  <Btn kind="ghost">ghost</Btn>
                  <Btn kind="dashed">dashed</Btn>
                </div>
              </div>
              <div>
                <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1, fontWeight: 700, marginBottom: 8 }}>PILLS</div>
                <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                  <Pill tone="neutral">neutral</Pill>
                  <Pill tone="solid" accent={T.c.wrong}>solid</Pill>
                  <Pill tone="soft" accent={T.c.act}>soft</Pill>
                  <Pill tone="ghost">ghost</Pill>
                </div>
              </div>
              <div>
                <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1, fontWeight: 700, marginBottom: 8 }}>CARD · COLUMN MEMBER</div>
                <Card accent={T.c.wrong} style={{ fontSize: 12 }}>
                  e2e flake. again. friday.
                  <CardFooter author="na" color="#cf4f4f" tag="flake" votes={4} />
                </Card>
              </div>
              <div>
                <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1, fontWeight: 700, marginBottom: 8 }}>AVATAR · STATUS</div>
                <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
                  <Avatar k="na" color="#cf4f4f" status="ready" />
                  <Avatar k="lu" color="#cf8a3f" status="writing" />
                  <Avatar k="sa" color="#8757b6" status="voting" />
                  <Avatar k="kt" color="#2f9469" status="away" />
                  <span style={{ marginLeft: 8 }}><Stack people={TEAM} size={24} /></span>
                </div>
              </div>
              <div>
                <div style={{ fontSize: 10.5, color: T.muted, letterSpacing: 1, fontWeight: 700, marginBottom: 8 }}>SHADOWS · ELEVATION</div>
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 8 }}>
                  {['s1','s2','s3'].map((s) => (
                    <div key={s} style={{
                      height: 50, borderRadius: 8, background: T.panelHi,
                      border: `1px solid ${T.line}`, boxShadow: T[s],
                      display: 'flex', alignItems: 'center', justifyContent: 'center',
                      fontSize: 11, color: T.fg2, fontWeight: 700,
                    }}>{s}</div>
                  ))}
                </div>
              </div>
            </Tile>
          </div>
        </div>
      </Frame>
    </DCArtboard>
  );
};

const Section = ({ label }) => (
  <div style={{ fontSize: 10.5, color: T.fg, letterSpacing: 1.4, fontWeight: 800, marginBottom: 10 }}>{label}</div>
);

})();
