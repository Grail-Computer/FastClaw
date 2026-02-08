import { animate, createTimeline, stagger } from 'https://esm.sh/animejs@4.1.2';
import { VARIANTS } from './variants.js';

const qs = (sel) => document.querySelector(sel);

let activeTimeline = null;
let runId = 0;
let doneTimer = null;
let typingAnim = null;

function byId(id) {
  return VARIANTS.find((v) => v.id === id) || VARIANTS[0];
}

function getVariantFromUrl() {
  const u = new URL(window.location.href);
  const v = u.searchParams.get('variant');
  return byId(v);
}

function shouldAutoplay() {
  const u = new URL(window.location.href);
  return u.searchParams.get('autoplay') === '1';
}

function el(tag, attrs = {}, children = []) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === 'class') node.className = v;
    else if (k === 'text') node.textContent = v;
    else if (k.startsWith('data-')) node.setAttribute(k, v);
    else node.setAttribute(k, v);
  }
  for (const c of children) node.appendChild(c);
  return node;
}

function renderInlineText(container, text) {
  // Very small "Slack-ish" formatter:
  // - `code`
  // - @mentions (pill)
  // - #channels (link style)
  // - urls (link style)
  const parts = String(text).split(/(`[^`]+`)/g).filter(Boolean);

  for (const part of parts) {
    if (part.startsWith('`') && part.endsWith('`') && part.length >= 2) {
      container.appendChild(el('code', { class: 'inline-code', text: part.slice(1, -1) }));
      continue;
    }

    let i = 0;
    const s = part;
    const re = /@[\w-]+|#[\w-]+|\bhttps?:\/\/[^\s]+/g;
    for (const m of s.matchAll(re)) {
      const start = m.index ?? 0;
      if (start > i) container.appendChild(document.createTextNode(s.slice(i, start)));

      const tok = m[0];
      if (tok.startsWith('@')) {
        container.appendChild(el('span', { class: 'mention', text: tok }));
      } else if (tok.startsWith('#')) {
        const name = tok.slice(1);
        const wrap = el('span', { class: 'channel-link' }, [
          document.createTextNode('#'),
          document.createTextNode(name),
        ]);
        container.appendChild(wrap);
      } else {
        container.appendChild(el('a', { href: tok, text: tok, target: '_blank', rel: 'noreferrer' }));
      }
      i = start + tok.length;
    }
    if (i < s.length) container.appendChild(document.createTextNode(s.slice(i)));
  }
}

function renderBlocks(blocks = []) {
  const wrap = el('div', { class: 'blocks' });
  for (const b of blocks) {
    if (b.type === 'section') {
      const box = el('div', { class: 'block' });
      renderInlineText(box, b.text || '');
      wrap.appendChild(box);
      continue;
    }

    if (b.type === 'list') {
      const box = el('div', { class: 'block' });
      if (b.title) box.appendChild(el('div', { class: 'name', text: b.title }));
      const ul = el('ul');
      for (const item of b.items || []) {
        const li = el('li');
        renderInlineText(li, item);
        ul.appendChild(li);
      }
      box.appendChild(ul);
      wrap.appendChild(box);
      continue;
    }

    if (b.type === 'code') {
      const pre = el('pre', { class: 'block block-code' });
      pre.textContent = b.text || '';
      wrap.appendChild(pre);
      continue;
    }

    if (b.type === 'buttons') {
      const box = el('div', { class: 'block block-actions' });
      for (const btn of b.buttons || []) {
        const cls =
          btn.variant === 'primary'
            ? 'slack-btn primary'
            : btn.variant === 'danger'
              ? 'slack-btn danger'
              : 'slack-btn';
        const bEl = el('button', { class: cls, type: 'button', 'data-btn': btn.id }, []);
        bEl.textContent = btn.label || 'Button';
        box.appendChild(bEl);
      }
      wrap.appendChild(box);
      continue;
    }
  }
  return wrap;
}

function renderFiles(files = []) {
  const wrap = el('div', { class: 'files' });
  for (const f of files) {
    const card = el('div', { class: 'file-card' });
    card.appendChild(el('div', { class: 'file-ico', text: (f.ext || 'file').slice(0, 4) }));
    const meta = el('div');
    meta.appendChild(el('div', { class: 'file-name', text: f.name || 'file' }));
    meta.appendChild(el('div', { class: 'file-sub', text: f.sub || '' }));
    card.appendChild(meta);
    wrap.appendChild(card);
  }
  return wrap;
}

function renderMessage(step, index) {
  const isBot = step.from === 'bot';
  const msg = el('div', { class: 'slack-msg step', 'data-step': `s${index}` });
  const av = el('div', { class: `avatar ${isBot ? 'bot' : 'user'}`, text: step.avatar || '?' });

  const body = el('div', { class: 'body' });
  const meta = el('div', { class: 'meta' });
  meta.appendChild(el('span', { class: 'name', text: step.name || (isBot ? 'μEmployee' : 'User') }));
  meta.appendChild(el('span', { class: 'time', text: step.time || '' }));
  body.appendChild(meta);

  const text = el('div', { class: 'text' });
  renderInlineText(text, step.text || '');
  body.appendChild(text);

  if (step.blocks?.length) body.appendChild(renderBlocks(step.blocks));
  if (step.files?.length) body.appendChild(renderFiles(step.files));

  msg.appendChild(av);
  msg.appendChild(body);
  return msg;
}

function renderTyping(step, index) {
  const wrap = el('div', { class: 'slack-typing step', 'data-step': `s${index}` });
  const av = el('div', { class: 'avatar bot', text: 'μ' });
  const bubble = el('div', { class: 'typing-bubble' });
  bubble.appendChild(el('span', { class: 'typing-dot' }));
  bubble.appendChild(el('span', { class: 'typing-dot' }));
  bubble.appendChild(el('span', { class: 'typing-dot' }));
  bubble.appendChild(el('span', { class: 'typing-label', text: `${step.who || 'μEmployee'} is typing...` }));
  wrap.appendChild(av);
  wrap.appendChild(bubble);
  return wrap;
}

function renderSidebar(listEl, items, { active = null, kind = 'channel' } = {}) {
  listEl.innerHTML = '';
  for (const name of items || []) {
    const isActive = active && name === active;
    if (kind === 'dm') {
      const item = el('div', { class: `sb-item dm ${isActive ? 'active' : ''}` });
      item.appendChild(el('span', { class: 'dot', 'aria-hidden': 'true' }));
      item.appendChild(el('span', { text: name }));
      listEl.appendChild(item);
    } else {
      const item = el('div', { class: `sb-item ${isActive ? 'active' : ''}` });
      item.appendChild(el('span', { class: 'hash', text: '#' }));
      item.appendChild(el('span', { text: name }));
      listEl.appendChild(item);
    }
  }
}

function hardReset() {
  // Cancel typing animation and hide cursor.
  try {
    typingAnim?.pause();
  } catch {
    // no-op
  }
  typingAnim = null;

  const title = qs('#title-card');
  if (title) {
    title.style.opacity = '0';
    title.style.transform = 'translate3d(0, 10px, 0)';
  }

  const cursor = qs('#cursor');
  if (cursor) {
    cursor.style.opacity = '0';
    cursor.style.transform = 'translate3d(0,0,0)';
    qs('#cursor .cursor-ring')?.style.setProperty('transform', 'scale(1)');
  }

  // Only hide message steps; keep the title overlay controlled by its own animation.
  for (const s of Array.from(document.querySelectorAll('[data-step]'))) {
    s.style.opacity = '0';
    s.style.transform = 'translate3d(0, 10px, 0)';
    // Use a CSS class (display:none !important) so anime can't accidentally unhide nodes
    // early by mutating inline styles; hidden nodes must not affect scroll height.
    s.classList.add('is-hidden');
    s.style.display = '';
  }
}

function placeCursorOver(elTarget) {
  const stage = qs('#stage');
  const cursor = qs('#cursor');
  if (!stage || !cursor || !elTarget) return { x: 0, y: 0 };

  const s = stage.getBoundingClientRect();
  const r = elTarget.getBoundingClientRect();
  const x = Math.round(r.left - s.left + r.width / 2 - 13);
  const y = Math.round(r.top - s.top + r.height / 2 - 13);
  cursor.style.transform = `translate3d(${x}px, ${y}px, 0)`;
  return { x, y };
}

function startTypingAnim(typingEl) {
  if (!typingEl) return null;
  const dots = Array.from(typingEl.querySelectorAll('.typing-dot'));
  if (!dots.length) return null;
  return animate(dots, {
    scale: [1, 1.7],
    opacity: [0.45, 1],
    delay: stagger(120),
    duration: 420,
    ease: 'inOutSine',
    direction: 'alternate',
    loop: true,
    autoplay: true,
  });
}

function scrollMessagesToBottom({ duration = 360 } = {}) {
  const msgRoot = qs('#messages');
  if (!msgRoot) return;

  // Keep the latest message visible without showing scrollbars.
  const max = msgRoot.scrollHeight - msgRoot.clientHeight;
  if (max <= 0) return;

  const from = msgRoot.scrollTop;
  const to = max;
  if (Math.abs(to - from) < 2) return;

  if (duration <= 0) {
    msgRoot.scrollTop = to;
    return;
  }

  const o = { v: from };
  animate(o, {
    v: to,
    duration,
    ease: 'outCubic',
    onUpdate: () => {
      msgRoot.scrollTop = o.v;
    },
    onComplete: () => {
      msgRoot.scrollTop = to;
    },
  });
}

function renderVariant(v) {
  qs('#ws-name').textContent = v.workspace || 'acme';
  qs('#channel-name').textContent = v.channel || 'channel';
  qs('#channel-topic').textContent = v.topic || '';
  qs('#composer-channel').textContent = v.channel || 'channel';
  qs('#variant-pill').textContent = v.id;
  qs('#corner-label').textContent = v.id;
  qs('#title-card-text').textContent = v.title || v.id;

  renderSidebar(qs('#sidebar-channels'), v.sidebarChannels, { active: v.channel, kind: 'channel' });
  renderSidebar(qs('#sidebar-dms'), v.sidebarDms, { active: 'μEmployee', kind: 'dm' });

  const msgRoot = qs('#messages');
  msgRoot.innerHTML = '';
  msgRoot.scrollTop = 0;

  const rendered = [];
  let i = 1;
  for (const step of v.steps || []) {
    if (step.kind === 'typing') {
      const node = renderTyping(step, i++);
      msgRoot.appendChild(node);
      rendered.push({ step, node });
      continue;
    }
    if (step.kind === 'msg') {
      const node = renderMessage(step, i++);
      msgRoot.appendChild(node);
      rendered.push({ step, node });
      continue;
    }
    // click steps don't render DOM nodes.
    rendered.push({ step, node: null, index: i++ });
  }

  return rendered;
}

function playDemo() {
  const v = getVariantFromUrl();
  const myRun = ++runId;

  // Stop any existing timeline to keep replays deterministic.
  if (activeTimeline) {
    try {
      activeTimeline.cancel();
    } catch {
      // no-op
    }
  }
  doneTimer && clearTimeout(doneTimer);

  window.__DEMO_DONE__ = false;

  const rendered = renderVariant(v);
  hardReset();

  const markDone = (resolve) => {
    if (myRun !== runId) return;
    if (window.__DEMO_DONE__ === true) return;
    window.__DEMO_DONE__ = true;
    resolve();
  };

  return new Promise((resolve) => {
    const tl = createTimeline({
      autoplay: false,
      defaults: { duration: 520, ease: 'outCubic' },
    });
    activeTimeline = tl;

    // Title card
    tl.add('#title-card', { opacity: [0, 1], translateY: [10, 0] }, 0).add(
      '#title-card',
      { opacity: [1, 0], translateY: [0, -6], duration: 420, ease: 'inQuad' },
      '+=920',
    );

    let cursorVisible = false;

    // Steps
    for (const entry of rendered) {
      const step = entry.step;
      if (step.kind === 'msg' && entry.node) {
        tl.add(
          entry.node,
          {
            opacity: [0, 1],
            translateY: [10, 0],
            onBegin: () => {
              entry.node.classList.remove('is-hidden');
            },
          },
          '+=120',
        ).call(() => {
          scrollMessagesToBottom({ duration: 420 });
        });
        continue;
      }

      if (step.kind === 'typing' && entry.node) {
        tl.add(
          entry.node,
          {
            opacity: [0, 1],
            translateY: [10, 0],
            onBegin: () => {
              entry.node.classList.remove('is-hidden');
            },
          },
          '+=120',
        )
          .call(() => scrollMessagesToBottom({ duration: 420 }))
          .call(() => {
            try {
              typingAnim?.pause();
            } catch {
              // no-op
            }
            typingAnim = startTypingAnim(entry.node);
          })
          .add({}, { duration: step.duration || 700 })
          .add(entry.node, { opacity: [1, 0], translateY: [0, -4], duration: 220, ease: 'inQuad' })
          .call(() => {
            try {
              typingAnim?.pause();
            } catch {
              // no-op
            }
            typingAnim = null;
            // Remove from layout so it doesn't leave a blank gap between messages.
            entry.node?.remove();
            scrollMessagesToBottom({ duration: 260 });
          });
        continue;
      }

      if (step.kind === 'click') {
        const cursor = qs('#cursor');
        const ring = qs('#cursor .cursor-ring');
        const btn = document.querySelector(`[data-btn="${step.buttonId}"]`);

        // Show cursor once we need it.
        if (!cursorVisible && cursor) {
          cursorVisible = true;
          tl.add(cursor, { opacity: [0, 1], duration: 180, ease: 'outQuad' }, '+=200');
        }

        tl.call(() => {
          if (!cursor || !btn) return;
          placeCursorOver(btn);
        })
          .add(cursor, { translateX: [0, 0], translateY: [0, 0], duration: 1 }, '+=1')
          .add(
            cursor,
            {
              duration: 420,
              ease: 'outCubic',
              update: () => {
                // Cursor is positioned by style transform set in call(); nothing else to update.
              },
            },
            '+=1',
          )
          .call(() => {
            if (ring) ring.style.transform = 'scale(0.86)';
            if (btn) btn.classList.add('selected');
            if (step.resultText && btn) btn.textContent = step.resultText;
          })
          .add({}, { duration: 140 })
          .call(() => {
            if (ring) ring.style.transform = 'scale(1)';
          })
          .add({}, { duration: 180 });
      }
    }

    // Hold last frame briefly.
    tl.add({}, { duration: 1000 });
    tl.call(() => markDone(resolve));

    tl.play();

    // Fallback: end after ~18s (covers the longest variant).
    doneTimer = setTimeout(() => markDone(resolve), 18_000);
  });
}

window.__DEMO_READY__ = true;
window.__DEMO_DONE__ = false;
window.playDemo = playDemo;

qs('#replay')?.addEventListener('click', () => {
  playDemo();
});

// Initial render so the page isn't blank.
renderVariant(getVariantFromUrl());
hardReset();

if (shouldAutoplay()) {
  playDemo();
}
