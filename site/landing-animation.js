// @ts-check

/** @param {Element} element */
function center(element) {
  const rect = element.getBoundingClientRect();
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}

/** @param {{x: number, y: number}} point */
function cursorPosition(point) {
  return `translate3d(${point.x - 3}px, ${point.y - 3}px, 0)`;
}

/** @param {{x: number, y: number}} from @param {{x: number, y: number}} to */
function stickPosition(from, to) {
  const distance = Math.hypot(to.x - from.x, to.y - from.y) || 1;
  return `translate(${((to.x - from.x) / distance) * 6}px, ${((to.y - from.y) / distance) * 6}px)`;
}

/** @param {Keyframe} rest @param {Keyframe} pressed @param {number[]} times */
function presses(rest, pressed, times) {
  return [
    { ...rest, offset: 0 },
    ...times.flatMap((time) => [
      { ...rest, offset: time },
      { ...pressed, offset: time + 0.01 },
      { ...pressed, offset: time + 0.03 },
      { ...rest, offset: time + 0.0375 },
    ]),
    { ...rest, offset: 1 },
  ];
}

function initLandingAnimation() {
  const hero = document.querySelector(".hero");
  const cursor = document.querySelector(".demo-cursor");
  const stick = document.querySelector(".demo-stick");
  const leftThumb = document.querySelector(".demo-thumb-left");
  const rightThumb = document.querySelector(".demo-thumb-right");
  const buttonA = document.querySelector(".demo-button-a");
  const windows = document.querySelector('[data-demo-target="windows"]');
  const linux = document.querySelector('[data-demo-target="linux"]');
  if (
    !hero ||
    !(cursor instanceof SVGElement) ||
    !stick ||
    !leftThumb ||
    !rightThumb ||
    !buttonA ||
    !windows ||
    !linux
  )
    return;

  const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");
  /** @type {Animation[]} */
  let animations = [];
  let inView = false;
  let repositioning = false;
  let restartTimer = 0;
  /** @type {KeyframeAnimationOptions} */
  const timing = { duration: 8000, iterations: Infinity, fill: "both" };
  const ease = "cubic-bezier(.4, 0, .2, 1)";

  function cancelAnimations() {
    animations.forEach((animation) => animation.cancel());
    animations = [];
  }

  const buildAnimations = () => {
    const start = center(stick);
    const first = center(windows);
    const second = center(linux);
    const home = cursorPosition(start);
    animations.push(
      cursor.animate(
        [
          { offset: 0, transform: home, opacity: 0 },
          { offset: 0.1, transform: home, opacity: 1, easing: ease },
          { offset: 0.3, transform: cursorPosition(first), opacity: 1 },
          { offset: 0.4375, transform: cursorPosition(first), opacity: 1, easing: ease },
          { offset: 0.6375, transform: cursorPosition(second), opacity: 1 },
          { offset: 0.675, transform: cursorPosition(second), opacity: 1, easing: ease },
          { offset: 0.85, transform: home, opacity: 1 },
          { offset: 1, transform: home, opacity: 0 },
        ],
        timing,
      ),
    );

    const moves = [
      { offset: 0, from: start, to: start },
      { offset: 0.1, from: start, to: first },
      { offset: 0.28, from: start, to: first },
      { offset: 0.3, from: start, to: start },
      { offset: 0.4375, from: first, to: second },
      { offset: 0.6175, from: first, to: second },
      { offset: 0.6375, from: start, to: start },
      { offset: 0.675, from: second, to: start },
      { offset: 0.83, from: second, to: start },
      { offset: 0.85, from: start, to: start },
      { offset: 1, from: start, to: start },
    ];
    const stickFrames = moves.map(({ offset, from, to }) => ({
      offset,
      transform: stickPosition(from, to),
    }));
    animations.push(stick.animate(stickFrames, timing), leftThumb.animate(stickFrames, timing));
    animations.push(
      rightThumb.animate(
        presses(
          { transform: "translate(0, 0) scale(1)" },
          { transform: "translate(-12px, -11px) scale(.96)" },
          [0.3, 0.6375],
        ),
        timing,
      ),
    );
    animations.push(
      buttonA.animate(
        presses(
          { transform: "scale(1)", filter: "drop-shadow(0 0 0 transparent)" },
          { transform: "scale(.86)", filter: "drop-shadow(0 0 8px #8fffc7)" },
          [0.3, 0.6375],
        ),
        timing,
      ),
    );
    for (const [target, time] of /** @type {[Element, number][]} */ ([
      [windows, 0.3],
      [linux, 0.6375],
    ])) {
      animations.push(
        target.animate(
          presses(
            { boxShadow: "0 0 0 0 rgb(143 255 199 / 0%)" },
            { boxShadow: "0 0 0 3px rgb(143 255 199 / 70%), 0 0 24px rgb(143 255 199 / 25%)" },
            [time],
          ),
          timing,
        ),
      );
    }
  };

  const syncPlayback = () => {
    if (reducedMotion.matches) {
      cancelAnimations();
      cursor.setAttribute("hidden", "");
      return;
    }
    const visible = inView && !document.hidden && !repositioning;
    cursor.toggleAttribute("hidden", !visible);
    if (visible && animations.length === 0) buildAnimations();
    for (const animation of animations) {
      if (visible) animation.play();
      else animation.pause();
    }
  };

  function restartAfterLayout() {
    clearTimeout(restartTimer);
    repositioning = true;
    cancelAnimations();
    syncPlayback();
    restartTimer = window.setTimeout(() => {
      repositioning = false;
      syncPlayback();
    }, 150);
  }

  new IntersectionObserver(
    ([entry]) => {
      inView = entry.isIntersecting && entry.intersectionRatio >= 0.15;
      syncPlayback();
    },
    { threshold: [0, 0.15] },
  ).observe(hero);
  window.addEventListener("resize", restartAfterLayout);
  window.addEventListener("scroll", restartAfterLayout, { passive: true });
  document.addEventListener("visibilitychange", syncPlayback);
  reducedMotion.addEventListener("change", syncPlayback);
  document.fonts.ready.then(restartAfterLayout);
  syncPlayback();
}

initLandingAnimation();
