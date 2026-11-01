// @ts-check

/**
 * Gamepad Desktop — Interactive & Natural Landing Animation
 *
 * Features:
 * 1. Left Stick: Manual drag with mouse or touch to control the cursor with calibrated sensitivity.
 * 2. Right Stick: Manual drag up/down with mouse or touch to smoothly scroll the page!
 * 3. Simulated Hover: Virtual cursor triggers realistic hover states on all interactive page elements.
 * 4. Button A: Click or press A on controller to trigger the hovered element.
 * 5. Physical Authenticity: Auto-showcase stick deflection mirrors cursor velocity.
 * 6. Live Gamepad API: Full physical controller support for both sticks and buttons.
 */

function initLandingAnimation() {
  const hero = document.querySelector(".hero");
  const heroVisual = document.getElementById("heroVisual");
  const cursorContainer = document.getElementById("demoCursorContainer");
  const cursorRipple = document.getElementById("demoCursorRipple");

  const stickLeft = document.getElementById("demoStickLeft");
  const stickRight = document.getElementById("demoStickRight");
  const thumbLeft = document.getElementById("demoThumbLeft");
  const thumbRight = document.getElementById("demoThumbRight");

  const buttonA = document.getElementById("demoBtnA");
  const buttonAPulse = document.getElementById("demoBtnAPulse");
  const buttonB = document.getElementById("demoBtnB");
  const buttonX = document.getElementById("demoBtnX");
  const buttonY = document.getElementById("demoBtnY");
  const dpad = document.getElementById("demoDpad");
  const guideBtn = document.getElementById("demoGuideBtn");

  const windows = document.querySelector('[data-demo-target="windows"]');
  const linux = document.querySelector('[data-demo-target="linux"]');

  const controllerBadge = document.getElementById("controllerBadge");
  const controllerStatusText = document.getElementById("controllerStatusText");
  const interactiveHint = document.getElementById("interactiveHint");

  if (
    !hero ||
    !cursorContainer ||
    !stickLeft ||
    !thumbLeft ||
    !thumbRight ||
    !buttonA ||
    !windows ||
    !linux
  ) {
    return;
  }

  // Socket base circles for true neutral centers
  const stickLeftBase = stickLeft.previousElementSibling;
  const stickRightBase = stickRight?.previousElementSibling;

  const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");

  // State
  let inView = false;
  let tabVisible = !document.hidden;
  let lastTime = performance.now();
  let loopTime = 0;
  const LOOP_DURATION = 9200; // ms

  // Left Stick Dragging State
  let isDraggingStick = false;
  let dragDeflection = { x: 0, y: 0 };
  let stickSocketCenter = { x: 0, y: 0 };
  let lastManualInput = 0;

  // Right Stick Dragging State (Page Scroll)
  let isDraggingRightStick = false;
  let rightStickDeflection = { x: 0, y: 0 };
  let rightStickSocketCenter = { x: 0, y: 0 };

  // Velocity smoothing & response curve tuning
  let smoothVelX = 0;
  let smoothVelY = 0;
  const MAX_SPEED_PPS = 780; // Calibrated speed (fast yet controllable)
  const MAX_SCROLL_PPS = 1350; // Smooth page scroll speed

  // Gamepad State
  let isGamepadControlled = false;
  let lastGamepadInput = 0;
  let prevBtnA = false;

  // Shared User Cursor Coordinates (for manual drag & gamepad modes)
  let userCursor = { x: window.innerWidth * 0.5, y: window.innerHeight * 0.4 };

  // Simulated Hover Tracking
  /** @type {Element | null} */
  let currentSimHoverEl = null;

  // ---------------------------------------------------------------------------
  // Geometry & Physics Functions
  // ---------------------------------------------------------------------------

  /**
   * Returns viewport center of any element
   * @param {Element | null} el
   */
  function getCenter(el) {
    if (!el) return { x: 0, y: 0 };
    const rect = el.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
  }

  function getLeftStickSocketCenter() {
    return getCenter(stickLeftBase || stickLeft);
  }

  function getRightStickSocketCenter() {
    return getCenter(stickRightBase || stickRight);
  }

  function smootherstep(t) {
    return t * t * t * (t * (t * 6 - 15) + 10);
  }

  function smootherstepVelocity(t) {
    return 30 * t * t * (1 - t) * (1 - t);
  }

  /**
   * Non-linear response curve with deadzone (based on Gamepad Desktop physics)
   * High precision near center for aiming at buttons, smooth ramp to cruising speed
   * @param {number} val - input axis from -1 to 1
   * @param {number} deadzone - deadzone threshold
   */
  function applyAxisResponseCurve(val, deadzone = 0.08) {
    const abs = Math.abs(val);
    if (abs <= deadzone) return 0;
    const scaled = (abs - deadzone) / (1 - deadzone);
    return Math.sign(val) * Math.pow(scaled, 1.7);
  }

  function setStick(sx, sy) {
    stickLeft.style.transform = `translate(${sx.toFixed(2)}px, ${sy.toFixed(2)}px)`;
    thumbLeft.style.transform = `translate(${sx.toFixed(2)}px, ${sy.toFixed(2)}px) rotate(${(sx * 1.5).toFixed(1)}deg)`;
  }

  function resetStick() {
    stickLeft.style.transform = "translate(0px, 0px)";
    thumbLeft.style.transform = "translate(0px, 0px) rotate(0deg)";
  }

  function setRightStick(sx, sy) {
    if (!stickRight) return;
    stickRight.style.transform = `translate(${sx.toFixed(2)}px, ${sy.toFixed(2)}px)`;
  }

  function resetRightStick() {
    if (!stickRight) return;
    stickRight.style.transform = "translate(0px, 0px)";
  }

  function triggerButtonPulse() {
    if (!buttonAPulse) return;
    buttonAPulse.classList.remove("is-pulsing");
    void buttonAPulse.offsetWidth;
    buttonAPulse.classList.add("is-pulsing");
  }

  function triggerCursorRipple() {
    if (!cursorRipple) return;
    cursorRipple.classList.remove("is-pulsing");
    void cursorRipple.offsetWidth;
    cursorRipple.classList.add("is-pulsing");
  }

  // ---------------------------------------------------------------------------
  // Simulated Hover & Click System
  // ---------------------------------------------------------------------------
  /**
   * Updates simulated hover highlight on whichever element is under the cursor
   * @param {number} x
   * @param {number} y
   */
  function updateSimulatedHover(x, y) {
    if (cursorContainer.hasAttribute("hidden")) {
      clearSimulatedHover();
      return;
    }

    const target = document.elementFromPoint(x, y);
    // Only target elements that naturally have a hover state in the original site
    const interactive = target?.closest(
      'a, button, summary, .download-card, .button-primary, .setup-link'
    );

    const isInsideController = heroVisual && heroVisual.contains(target);

    if (interactive && !isInsideController) {
      if (interactive !== currentSimHoverEl) {
        if (currentSimHoverEl) currentSimHoverEl.classList.remove("is-sim-hover");
        interactive.classList.add("is-sim-hover");
        currentSimHoverEl = interactive;
      }
    } else {
      clearSimulatedHover();
    }
  }

  function clearSimulatedHover() {
    if (currentSimHoverEl) {
      currentSimHoverEl.classList.remove("is-sim-hover");
      currentSimHoverEl = null;
    }
  }

  /**
   * Performs an interactive click with the user cursor on whatever is hovered
   */
  function triggerUserClick() {
    triggerButtonPulse();
    triggerCursorRipple();

    cursorContainer.classList.add("is-clicking");
    setTimeout(() => cursorContainer.classList.remove("is-clicking"), 220);

    // If an element is hovered, trigger its action
    if (currentSimHoverEl) {
      const card = currentSimHoverEl.closest(".download-card");
      if (card) {
        card.classList.add("is-demo-active");
        setTimeout(() => card.classList.remove("is-demo-active"), 450);
        return;
      }

      // If it's an accordion summary
      if (currentSimHoverEl.tagName === "SUMMARY") {
        const details = currentSimHoverEl.closest("details");
        if (details) details.open = !details.open;
        return;
      }

      // If it's an anchor link with hash
      const href = currentSimHoverEl.getAttribute("href");
      if (href && href.startsWith("#")) {
        const targetSection = document.querySelector(href);
        if (targetSection) {
          targetSection.scrollIntoView({ behavior: "smooth" });
          return;
        }
      }

      // Generic click effect
      currentSimHoverEl.classList.add("is-demo-active");
      setTimeout(() => currentSimHoverEl?.classList.remove("is-demo-active"), 350);
    }
  }

  // ---------------------------------------------------------------------------
  // Left Stick: Manual Dragging (Cursor Movement)
  // ---------------------------------------------------------------------------
  /** @param {PointerEvent} e */
  function updateStickDrag(e) {
    const dx = e.clientX - stickSocketCenter.x;
    const dy = e.clientY - stickSocketCenter.y;
    const dist = Math.hypot(dx, dy);

    // Throw radius (45px provides comfortable throw)
    const maxDragRadius = 45;
    const clampedDist = Math.min(dist, maxDragRadius);
    const factor = dist > 0 ? clampedDist / dist : 0;

    const normX = (dx * factor) / maxDragRadius;
    const normY = (dy * factor) / maxDragRadius;

    dragDeflection.x = normX;
    dragDeflection.y = normY;

    setStick(normX * 8.5, normY * 8.5);
  }

  /** @param {PointerEvent} e */
  function onStickPointerDown(e) {
    if (reducedMotion.matches) return;
    e.preventDefault();

    isDraggingStick = true;
    stickSocketCenter = getLeftStickSocketCenter();

    try {
      stickLeft.setPointerCapture(e.pointerId);
    } catch (_) {}

    stickLeft.classList.add("is-dragging");
    updateStickDrag(e);

    if (!userCursor || isNaN(userCursor.x)) {
      userCursor = { x: stickSocketCenter.x, y: stickSocketCenter.y };
    }

    cursorContainer.removeAttribute("hidden");
    cursorContainer.style.opacity = "1";

    if (controllerBadge) controllerBadge.classList.add("is-gamepad");
    if (controllerStatusText) {
      controllerStatusText.textContent = "🕹️ Left Stick · Moving Cursor";
    }
    if (interactiveHint) {
      interactiveHint.textContent = "Release to Stop";
    }
  }

  /** @param {PointerEvent} e */
  function onStickPointerMove(e) {
    if (!isDraggingStick) return;
    e.preventDefault();
    updateStickDrag(e);
  }

  /** @param {PointerEvent} e */
  function onStickPointerUp(e) {
    if (!isDraggingStick) return;
    isDraggingStick = false;
    stickLeft.classList.remove("is-dragging");

    try {
      stickLeft.releasePointerCapture(e.pointerId);
    } catch (_) {}

    smoothVelX = 0;
    smoothVelY = 0;

    let springTime = 0;
    const startX = dragDeflection.x * 8.5;
    const startY = dragDeflection.y * 8.5;
    dragDeflection.x = 0;
    dragDeflection.y = 0;

    function recoilStep() {
      springTime += 16;
      const t = springTime / 1000;
      const decay = Math.exp(-t * 14);
      const osc = Math.cos(t * 32);
      const sx = startX * decay * osc;
      const sy = startY * decay * osc;

      setStick(sx, sy);

      if (t < 0.35 && !isDraggingStick && !isGamepadControlled) {
        requestAnimationFrame(recoilStep);
      } else if (!isDraggingStick && !isGamepadControlled) {
        resetStick();
      }
    }
    requestAnimationFrame(recoilStep);

    lastManualInput = performance.now();

    if (controllerStatusText) {
      controllerStatusText.textContent = "Press [A] to Click";
    }
  }

  stickLeft.addEventListener("pointerdown", onStickPointerDown);
  stickLeft.addEventListener("pointermove", onStickPointerMove);
  stickLeft.addEventListener("pointerup", onStickPointerUp);
  stickLeft.addEventListener("pointercancel", onStickPointerUp);

  // ---------------------------------------------------------------------------
  // Right Stick: Manual Dragging (Page Scroll)
  // ---------------------------------------------------------------------------
  if (stickRight) {
    let rightStickStartPointer = { x: 0, y: 0 };

    /** @param {PointerEvent} e */
    function updateRightStickDrag(e) {
      // Measure offset relative to starting pointer position (never drifts as page scrolls!)
      const dy = e.clientY - rightStickStartPointer.y;
      const dx = e.clientX - rightStickStartPointer.x;
      const dist = Math.hypot(dx, dy);

      const maxDragRadius = 40;
      const clampedDist = Math.min(dist, maxDragRadius);
      const factor = dist > 0 ? clampedDist / dist : 0;

      const normX = (dx * factor) / maxDragRadius;
      const normY = (dy * factor) / maxDragRadius;

      rightStickDeflection.x = normX;
      rightStickDeflection.y = normY;

      setRightStick(normX * 6, normY * 6);
    }

    /** @param {PointerEvent} e */
    function onRightStickPointerDown(e) {
      if (reducedMotion.matches) return;
      e.preventDefault();

      isDraggingRightStick = true;
      rightStickStartPointer = { x: e.clientX, y: e.clientY };
      // Disable html smooth-scroll queuing conflict while actively dragging the right stick
      document.documentElement.style.scrollBehavior = "auto";

      try {
        stickRight.setPointerCapture(e.pointerId);
      } catch (_) {}

      stickRight.classList.add("is-dragging");
      updateRightStickDrag(e);

      if (controllerBadge) controllerBadge.classList.add("is-gamepad");
      if (controllerStatusText) {
        controllerStatusText.textContent = "🕹️ Right Stick · Scrolling Page";
      }
    }

    /** @param {PointerEvent} e */
    function onRightStickPointerMove(e) {
      if (!isDraggingRightStick) return;
      e.preventDefault();
      updateRightStickDrag(e);
    }

    /** @param {PointerEvent} e */
    function onRightStickPointerUp(e) {
      if (!isDraggingRightStick) return;
      isDraggingRightStick = false;
      stickRight.classList.remove("is-dragging");
      document.documentElement.style.scrollBehavior = "";

      try {
        stickRight.releasePointerCapture(e.pointerId);
      } catch (_) {}

      let springTime = 0;
      const startX = rightStickDeflection.x * 6;
      const startY = rightStickDeflection.y * 6;
      rightStickDeflection.x = 0;
      rightStickDeflection.y = 0;

      function recoilRightStep() {
        springTime += 16;
        const t = springTime / 1000;
        const decay = Math.exp(-t * 14);
        const osc = Math.cos(t * 32);
        const sx = startX * decay * osc;
        const sy = startY * decay * osc;

        setRightStick(sx, sy);

        if (t < 0.35 && !isDraggingRightStick && !isGamepadControlled) {
          requestAnimationFrame(recoilRightStep);
        } else if (!isDraggingRightStick && !isGamepadControlled) {
          resetRightStick();
        }
      }
      requestAnimationFrame(recoilRightStep);

      lastManualInput = performance.now();
      if (controllerStatusText) {
        controllerStatusText.textContent = "Desktop Controller Demo";
      }
    }

    stickRight.addEventListener("pointerdown", onRightStickPointerDown);
    stickRight.addEventListener("pointermove", onRightStickPointerMove);
    stickRight.addEventListener("pointerup", onRightStickPointerUp);
    stickRight.addEventListener("pointercancel", onRightStickPointerUp);
  }

  // ---------------------------------------------------------------------------
  // 3D Parallax on Hero Visual Card (Mouse Hover)
  // ---------------------------------------------------------------------------
  if (heroVisual) {
    heroVisual.addEventListener("mousemove", (e) => {
      if (reducedMotion.matches || isDraggingStick || isDraggingRightStick) return;
      const rect = heroVisual.getBoundingClientRect();
      const normX = (e.clientX - rect.left) / rect.width - 0.5;
      const normY = (e.clientY - rect.top) / rect.height - 0.5;

      heroVisual.style.transform = `perspective(900px) rotateX(${(-normY * 9).toFixed(2)}deg) rotateY(${(normX * 9).toFixed(2)}deg)`;
      heroVisual.style.setProperty("--shine-x", `${((normX + 0.5) * 100).toFixed(1)}%`);
      heroVisual.style.setProperty("--shine-y", `${((normY + 0.5) * 100).toFixed(1)}%`);
    });

    heroVisual.addEventListener("mouseleave", () => {
      heroVisual.style.transform = "";
    });
  }

  // ---------------------------------------------------------------------------
  // Direct Click / Touch on SVG Controller Buttons
  // ---------------------------------------------------------------------------
  /**
   * @param {Element | null} el
   * @param {string} badgeText
   */
  function attachButtonPress(el, badgeText) {
    if (!el) return;
    const press = () => {
      el.classList.add("is-pressed");
      if (el === buttonA) {
        triggerUserClick();
      }
      if (controllerStatusText && badgeText) {
        controllerStatusText.textContent = badgeText;
      }
      lastManualInput = performance.now();
    };
    const release = () => {
      el.classList.remove("is-pressed");
      if (controllerStatusText && !isGamepadControlled && !isDraggingStick && !isDraggingRightStick) {
        controllerStatusText.textContent = "Desktop Controller Demo";
      }
    };

    el.addEventListener("mousedown", press);
    window.addEventListener("mouseup", release);
    el.addEventListener("touchstart", press, { passive: true });
    window.addEventListener("touchend", release);
  }

  attachButtonPress(buttonA, "Button A · Click");
  attachButtonPress(buttonB, "Button B · Right-Click");
  attachButtonPress(buttonX, "Button X");
  attachButtonPress(buttonY, "Button Y");
  attachButtonPress(dpad, "D-Pad");
  attachButtonPress(guideBtn, "Home Guide");

  // ---------------------------------------------------------------------------
  // Physical Gamepad API Support
  // ---------------------------------------------------------------------------
  function pollGamepad(delta) {
    if (reducedMotion.matches) return false;
    const gamepads = navigator.getGamepads ? navigator.getGamepads() : [];
    let gp = null;
    for (let i = 0; i < gamepads.length; i++) {
      if (gamepads[i] && gamepads[i].connected) {
        gp = gamepads[i];
        break;
      }
    }

    if (!gp) {
      if (isGamepadControlled && performance.now() - lastGamepadInput > 4500) {
        isGamepadControlled = false;
        controllerBadge?.classList.remove("is-gamepad");
        if (controllerStatusText) controllerStatusText.textContent = "Desktop Controller Demo";
        if (interactiveHint) interactiveHint.textContent = "🕹️ Drag Stick or Gamepad";
      }
      return false;
    }

    const ax = Math.abs(gp.axes[0]) > 0.12 ? gp.axes[0] : 0;
    const ay = Math.abs(gp.axes[1]) > 0.12 ? gp.axes[1] : 0;
    const rx = Math.abs(gp.axes[2]) > 0.14 ? gp.axes[2] : 0;
    const ry = Math.abs(gp.axes[3]) > 0.14 ? gp.axes[3] : 0;

    const pressedA = Boolean(gp.buttons[0]?.pressed);
    const pressedB = Boolean(gp.buttons[1]?.pressed);
    const pressedX = Boolean(gp.buttons[2]?.pressed);
    const pressedY = Boolean(gp.buttons[3]?.pressed);
    const pressedDpad = gp.buttons.slice(12, 16).some((b) => b?.pressed);

    const hasInput = ax !== 0 || ay !== 0 || rx !== 0 || ry !== 0 || pressedA || pressedB || pressedX || pressedY || pressedDpad;

    if (hasInput) {
      isGamepadControlled = true;
      lastGamepadInput = performance.now();

      const gpName = gp.id.replace(/\(.*\)/, "").trim() || "Gamepad";
      controllerBadge?.classList.add("is-gamepad");
      if (controllerStatusText) {
        controllerStatusText.textContent = `🎮 ${gpName.slice(0, 18)} Active`;
      }
      if (interactiveHint) {
        interactiveHint.textContent = "Controlling Desktop";
      }
    }

    if (!isGamepadControlled) {
      controllerBadge?.classList.add("is-gamepad");
      if (controllerStatusText) {
        controllerStatusText.textContent = "🎮 Gamepad Connected · Move Stick";
      }
      return false;
    }

    // Mirror physical left stick
    setStick(ax * 8.5, ay * 8.5);

    // Mirror physical right stick & scroll page
    setRightStick(rx * 6, ry * 6);
    if (ry !== 0) {
      const dtSeconds = delta / 1000;
      const scrollAmt = applyAxisResponseCurve(ry, 0.05) * MAX_SCROLL_PPS * dtSeconds;
      if (Math.abs(scrollAmt) > 0.05) {
        window.scrollBy({ top: scrollAmt, behavior: "instant" });
      }
    }

    buttonA.classList.toggle("is-pressed", pressedA);
    buttonB?.classList.toggle("is-pressed", pressedB);
    buttonX?.classList.toggle("is-pressed", pressedX);
    buttonY?.classList.toggle("is-pressed", pressedY);
    dpad?.classList.toggle("is-pressed", pressedDpad);

    if (thumbRight) {
      thumbRight.style.transform = pressedA
        ? "translate(-12px, -11px) scale(0.95)"
        : "translate(0px, 0px) scale(1)";
    }

    // Move cursor across viewport with response curve & delta time
    const dtSeconds = delta / 1000;
    const targetVelX = applyAxisResponseCurve(ax) * MAX_SPEED_PPS;
    const targetVelY = applyAxisResponseCurve(ay) * MAX_SPEED_PPS;

    smoothVelX += (targetVelX - smoothVelX) * 0.35;
    smoothVelY += (targetVelY - smoothVelY) * 0.35;

    userCursor.x = Math.max(10, Math.min(window.innerWidth - 30, userCursor.x + smoothVelX * dtSeconds));
    userCursor.y = Math.max(10, Math.min(window.innerHeight - 30, userCursor.y + smoothVelY * dtSeconds));

    cursorContainer.removeAttribute("hidden");
    cursorContainer.style.opacity = "1";
    cursorContainer.style.transform = `translate3d(${(userCursor.x - 3).toFixed(1)}px, ${(userCursor.y - 3).toFixed(1)}px, 0)`;
    cursorContainer.classList.toggle("is-clicking", pressedA);

    updateSimulatedHover(userCursor.x, userCursor.y);

    if (pressedA && !prevBtnA) {
      triggerUserClick();
    }
    prevBtnA = pressedA;

    return true;
  }

  // ---------------------------------------------------------------------------
  // Fluid & Natural Showcase Animation
  // ---------------------------------------------------------------------------
  let clickedThisSegment = false;
  let activeCard = null;

  function runShowcase(delta) {
    loopTime = (loopTime + delta) % LOOP_DURATION;
    const sec = loopTime / 1000;

    const stickCenter = getLeftStickSocketCenter();
    const winCenter = getCenter(windows);
    const linuxCenter = getCenter(linux);

    let curX = stickCenter.x;
    let curY = stickCenter.y;
    let opacity = 1;
    let sx = 0;
    let sy = 0;
    let clicking = false;

    const interpolateMotion = (from, to, tNorm, arcAmount) => {
      const p = smootherstep(tNorm);
      const dx = to.x - from.x;
      const dy = to.y - from.y;
      const dist = Math.hypot(dx, dy) || 1;
      const dirX = dx / dist;
      const dirY = dy / dist;

      const arc = Math.sin(tNorm * Math.PI) * arcAmount;
      const x = from.x + dx * p - dirY * arc;
      const y = from.y + dy * p + dirX * arc;

      const speedNorm = smootherstepVelocity(tNorm) / 1.875;
      const maxTilt = 8.5;
      const stickTiltX = dirX * speedNorm * maxTilt;
      const stickTiltY = dirY * speedNorm * maxTilt;

      return { x, y, stickTiltX, stickTiltY };
    };

    // -------------------------------------------------------------------------
    // Timeline Sequence:
    // 0.0s - 0.7s: Wake up at stick center, fade in
    // 0.7s - 2.8s: Glide from Stick to "Download for Windows" (Travels LEFT)
    // 2.8s - 4.1s: Settle on Windows button, spring recoil, tactile click
    // 4.1s - 5.7s: Glide from Windows button to "Download for Linux" (Travels RIGHT)
    // 5.7s - 7.0s: Settle on Linux button, spring recoil, tactile click
    // 7.0s - 8.6s: Return glide back to Stick (Travels RIGHT)
    // 8.6s - 9.2s: Settle at Stick, fade out
    // -------------------------------------------------------------------------

    if (sec < 0.7) {
      const tNorm = sec / 0.7;
      curX = stickCenter.x;
      curY = stickCenter.y;
      opacity = Math.min(1, tNorm / 0.5);
      sx = 0;
      sy = 0;
      clickedThisSegment = false;
      if (activeCard) {
        activeCard.classList.remove("is-demo-active");
        activeCard = null;
      }
    } else if (sec < 2.8) {
      const tNorm = (sec - 0.7) / 2.1;
      const motion = interpolateMotion(stickCenter, winCenter, tNorm, -18);
      curX = motion.x;
      curY = motion.y;
      sx = motion.stickTiltX;
      sy = motion.stickTiltY;
      clickedThisSegment = false;
    } else if (sec < 4.1) {
      const tNorm = (sec - 2.8) / 1.3;
      curX = winCenter.x;
      curY = winCenter.y;

      const decay = Math.exp(-tNorm * 9);
      sx = decay * Math.sin(tNorm * Math.PI * 4) * 2;
      sy = 0;

      if (tNorm >= 0.2 && tNorm <= 0.65) {
        clicking = true;
        windows.classList.add("is-demo-active");
        activeCard = windows;

        if (!clickedThisSegment) {
          triggerButtonPulse();
          triggerCursorRipple();
          clickedThisSegment = true;
        }
      } else {
        windows.classList.remove("is-demo-active");
      }
    } else if (sec < 5.7) {
      const tNorm = (sec - 4.1) / 1.6;
      const motion = interpolateMotion(winCenter, linuxCenter, tNorm, 14);
      curX = motion.x;
      curY = motion.y;
      sx = motion.stickTiltX;
      sy = motion.stickTiltY;
      clickedThisSegment = false;
      windows.classList.remove("is-demo-active");
    } else if (sec < 7.0) {
      const tNorm = (sec - 5.7) / 1.3;
      curX = linuxCenter.x;
      curY = linuxCenter.y;

      const decay = Math.exp(-tNorm * 9);
      sx = decay * Math.sin(tNorm * Math.PI * 4) * 2;
      sy = 0;

      if (tNorm >= 0.2 && tNorm <= 0.65) {
        clicking = true;
        linux.classList.add("is-demo-active");
        activeCard = linux;

        if (!clickedThisSegment) {
          triggerButtonPulse();
          triggerCursorRipple();
          clickedThisSegment = true;
        }
      } else {
        linux.classList.remove("is-demo-active");
      }
    } else if (sec < 8.6) {
      const tNorm = (sec - 7.0) / 1.6;
      const motion = interpolateMotion(linuxCenter, stickCenter, tNorm, -14);
      curX = motion.x;
      curY = motion.y;
      sx = motion.stickTiltX;
      sy = motion.stickTiltY;
      clickedThisSegment = false;
      linux.classList.remove("is-demo-active");
    } else {
      const tNorm = (sec - 8.6) / 0.6;
      curX = stickCenter.x;
      curY = stickCenter.y;
      opacity = Math.max(0, 1 - tNorm);
      sx = 0;
      sy = 0;
    }

    setStick(sx, sy);

    cursorContainer.removeAttribute("hidden");
    cursorContainer.style.opacity = opacity.toFixed(3);
    cursorContainer.style.transform = `translate3d(${(curX - 3).toFixed(1)}px, ${(curY - 3).toFixed(1)}px, 0)`;

    buttonA.classList.toggle("is-pressed", clicking);
    cursorContainer.classList.toggle("is-clicking", clicking);

    if (thumbRight) {
      thumbRight.style.transform = clicking
        ? "translate(-12px, -11px) scale(0.95)"
        : "translate(0px, 0px) scale(1)";
    }

    // Dynamic hover on whichever element is currently hovered by the auto cursor
    updateSimulatedHover(curX, curY);
  }

  // ---------------------------------------------------------------------------
  // Main Animation Loop
  // ---------------------------------------------------------------------------
  function frame(timestamp) {
    const delta = Math.min(64, timestamp - lastTime);
    lastTime = timestamp;

    const visible = inView && tabVisible && !reducedMotion.matches;

    if (visible) {
      // 1. Right stick continuous scroll if dragged
      if (isDraggingRightStick) {
        const dtSeconds = delta / 1000;
        const scrollAmt = applyAxisResponseCurve(rightStickDeflection.y, 0.05) * MAX_SCROLL_PPS * dtSeconds;
        if (Math.abs(scrollAmt) > 0.05) {
          window.scrollBy({ top: scrollAmt, behavior: "instant" });
        }
      }

      // 2. Left stick manual cursor control if dragged
      if (isDraggingStick) {
        const dtSeconds = delta / 1000;
        const targetVelX = applyAxisResponseCurve(dragDeflection.x) * MAX_SPEED_PPS;
        const targetVelY = applyAxisResponseCurve(dragDeflection.y) * MAX_SPEED_PPS;

        smoothVelX += (targetVelX - smoothVelX) * 0.40;
        smoothVelY += (targetVelY - smoothVelY) * 0.40;

        userCursor.x = Math.max(10, Math.min(window.innerWidth - 30, userCursor.x + smoothVelX * dtSeconds));
        userCursor.y = Math.max(10, Math.min(window.innerHeight - 30, userCursor.y + smoothVelY * dtSeconds));

        cursorContainer.removeAttribute("hidden");
        cursorContainer.style.opacity = "1";
        cursorContainer.style.transform = `translate3d(${(userCursor.x - 3).toFixed(1)}px, ${(userCursor.y - 3).toFixed(1)}px, 0)`;

        updateSimulatedHover(userCursor.x, userCursor.y);
      } else {
        const handledByGamepad = pollGamepad(delta);
        if (!handledByGamepad) {
          // If recently used manually, keep cursor visible for 3.5s before resuming auto demo
          if (performance.now() - lastManualInput < 3500) {
            cursorContainer.removeAttribute("hidden");
            cursorContainer.style.opacity = "1";
            cursorContainer.style.transform = `translate3d(${(userCursor.x - 3).toFixed(1)}px, ${(userCursor.y - 3).toFixed(1)}px, 0)`;
            updateSimulatedHover(userCursor.x, userCursor.y);
          } else {
            runShowcase(delta);
          }
        }
      }
    } else {
      cursorContainer.setAttribute("hidden", "");
      clearSimulatedHover();
      resetStick();
      resetRightStick();
    }

    requestAnimationFrame(frame);
  }

  // ---------------------------------------------------------------------------
  // Observers & Viewport Events
  // ---------------------------------------------------------------------------
  const observer = new IntersectionObserver(
    ([entry]) => {
      inView = entry.isIntersecting && entry.intersectionRatio >= 0.12;
      if (!inView) {
        windows.classList.remove("is-demo-active");
        linux.classList.remove("is-demo-active");
        cursorContainer.classList.remove("is-clicking");
        buttonA.classList.remove("is-pressed");
        clearSimulatedHover();
        resetStick();
        resetRightStick();
      }
    },
    { threshold: [0, 0.12, 0.5] }
  );

  observer.observe(hero);

  document.addEventListener("visibilitychange", () => {
    tabVisible = !document.hidden;
    lastTime = performance.now();
  });

  reducedMotion.addEventListener("change", () => {
    if (reducedMotion.matches) {
      cursorContainer.setAttribute("hidden", "");
      clearSimulatedHover();
      resetStick();
      resetRightStick();
    }
  });

  // Start RAF loop
  requestAnimationFrame(frame);
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initLandingAnimation);
} else {
  initLandingAnimation();
}
