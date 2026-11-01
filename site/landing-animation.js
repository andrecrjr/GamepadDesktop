// @ts-check

/**
 * Gamepad Desktop — Refined Natural & Fluid Landing Animation
 *
 * Core Principles:
 * 1. Physical authenticity: Analog stick deflection is directly proportional to
 *    cursor velocity (stick pushes -> cursor moves, stick returns -> cursor stops).
 * 2. Natural trajectories: Organic Hermite curves (smootherstep) with subtle arcs.
 * 3. Tactile feedback: Thumb flexing, spring recoil, clean click ripples, button depression.
 * 4. Absolute precision: Coordinates dynamically sampled in viewport space.
 * 5. Real Gamepad API: Physical controllers seamlessly take live control.
 */

function initLandingAnimation() {
  const hero = document.querySelector(".hero");
  const heroVisual = document.getElementById("heroVisual");
  const cursorContainer = document.getElementById("demoCursorContainer");
  const cursorRipple = document.getElementById("demoCursorRipple");
  const cursorSvg = cursorContainer?.querySelector(".demo-cursor");

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

  // The base circle under the stick is stationary and gives the true neutral socket center
  const stickBase = stickLeft.previousElementSibling;

  const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");

  // State
  let inView = false;
  let tabVisible = !document.hidden;
  let lastTime = performance.now();
  let loopTime = 0;
  const LOOP_DURATION = 9200; // ms

  // Gamepad state
  let isGamepadControlled = false;
  let lastGamepadInput = 0;
  let userCursor = { x: window.innerWidth * 0.5, y: window.innerHeight * 0.4 };
  let prevBtnA = false;

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

  /**
   * True socket center of the left analog stick in viewport space
   */
  function getStickSocketCenter() {
    return getCenter(stickBase || stickLeft);
  }

  /**
   * Smootherstep curve (Quintic polynomial: 6t^5 - 15t^4 + 10t^3)
   * Smooth zero velocity & zero acceleration at both t=0 and t=1
   */
  function smootherstep(t) {
    return t * t * t * (t * (t * 6 - 15) + 10);
  }

  /**
   * Derivative of smootherstep: velocity profile (30t^2 * (1-t)^2)
   * Peaks smoothly at t = 0.5 with peak value 1.875
   */
  function smootherstepVelocity(t) {
    return 30 * t * t * (1 - t) * (1 - t);
  }

  /**
   * Sets analog stick and left thumb deflection
   * @param {number} sx - deflection in pixels (-9 to 9)
   * @param {number} sy - deflection in pixels (-9 to 9)
   */
  function setStick(sx, sy) {
    stickLeft.style.transform = `translate(${sx.toFixed(2)}px, ${sy.toFixed(2)}px)`;
    // The thumb rests over the stick and flexes in the tilt direction
    thumbLeft.style.transform = `translate(${sx.toFixed(2)}px, ${sy.toFixed(2)}px) rotate(${(sx * 1.5).toFixed(1)}deg)`;
  }

  function resetStick() {
    stickLeft.style.transform = "translate(0px, 0px)";
    thumbLeft.style.transform = "translate(0px, 0px) rotate(0deg)";
  }

  /**
   * Triggers the expanding neon pulse ring on Button A
   */
  function triggerButtonPulse() {
    if (!buttonAPulse) return;
    buttonAPulse.classList.remove("is-pulsing");
    void buttonAPulse.offsetWidth;
    buttonAPulse.classList.add("is-pulsing");
  }

  /**
   * Triggers the expanding soft click ripple on the cursor
   */
  function triggerCursorRipple() {
    if (!cursorRipple) return;
    cursorRipple.classList.remove("is-pulsing");
    void cursorRipple.offsetWidth;
    cursorRipple.classList.add("is-pulsing");
  }

  // ---------------------------------------------------------------------------
  // 3D Parallax on Hero Visual Card (Mouse Hover)
  // ---------------------------------------------------------------------------
  if (heroVisual) {
    heroVisual.addEventListener("mousemove", (e) => {
      if (reducedMotion.matches) return;
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
      if (el === buttonA) triggerButtonPulse();
      if (controllerStatusText && badgeText) {
        controllerStatusText.textContent = badgeText;
      }
    };
    const release = () => {
      el.classList.remove("is-pressed");
      if (controllerStatusText && !isGamepadControlled) {
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
  function pollGamepad() {
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
        if (interactiveHint) interactiveHint.textContent = "Gamepad Supported";
      }
      return false;
    }

    const ax = Math.abs(gp.axes[0]) > 0.14 ? gp.axes[0] : 0;
    const ay = Math.abs(gp.axes[1]) > 0.14 ? gp.axes[1] : 0;

    const pressedA = Boolean(gp.buttons[0]?.pressed);
    const pressedB = Boolean(gp.buttons[1]?.pressed);
    const pressedX = Boolean(gp.buttons[2]?.pressed);
    const pressedY = Boolean(gp.buttons[3]?.pressed);
    const pressedDpad = gp.buttons.slice(12, 16).some((b) => b?.pressed);

    const hasInput = ax !== 0 || ay !== 0 || pressedA || pressedB || pressedX || pressedY || pressedDpad;

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

    // Mirror physical stick to virtual SVG stick
    setStick(ax * 8.5, ay * 8.5);

    // Right stick
    if (stickRight) {
      const rx = Math.abs(gp.axes[2]) > 0.15 ? gp.axes[2] : 0;
      const ry = Math.abs(gp.axes[3]) > 0.15 ? gp.axes[3] : 0;
      stickRight.style.transform = `translate(${(rx * 6).toFixed(2)}px, ${(ry * 6).toFixed(2)}px)`;
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

    // Move cursor across viewport
    const speed = 19;
    userCursor.x = Math.max(10, Math.min(window.innerWidth - 30, userCursor.x + ax * speed));
    userCursor.y = Math.max(10, Math.min(window.innerHeight - 30, userCursor.y + ay * speed));

    cursorContainer.removeAttribute("hidden");
    cursorContainer.style.opacity = "1";
    cursorContainer.style.transform = `translate3d(${(userCursor.x - 3).toFixed(1)}px, ${(userCursor.y - 3).toFixed(1)}px, 0)`;
    cursorContainer.classList.toggle("is-clicking", pressedA);

    if (pressedA && !prevBtnA) {
      triggerButtonPulse();
      triggerCursorRipple();
      const targetEl = document.elementFromPoint(userCursor.x, userCursor.y);
      const card = targetEl?.closest(".download-card");
      if (card) {
        card.classList.add("is-demo-active");
        setTimeout(() => card.classList.remove("is-demo-active"), 400);
      }
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
    const sec = loopTime / 1000; // current time in seconds

    // Accurate real-time viewport coordinates
    const stickCenter = getStickSocketCenter();
    const winCenter = getCenter(windows);
    const linuxCenter = getCenter(linux);

    let curX = stickCenter.x;
    let curY = stickCenter.y;
    let opacity = 1;
    let sx = 0;
    let sy = 0;
    let clicking = false;

    // Helper for smooth trajectory segment
    const interpolateMotion = (from, to, tNorm, arcAmount) => {
      const p = smootherstep(tNorm);
      const dx = to.x - from.x;
      const dy = to.y - from.y;
      const dist = Math.hypot(dx, dy) || 1;
      const dirX = dx / dist;
      const dirY = dy / dist;

      // Gentle perpendicular arc gives organic wrist-like sweeping
      const arc = Math.sin(tNorm * Math.PI) * arcAmount;
      const x = from.x + dx * p - dirY * arc;
      const y = from.y + dy * p + dirX * arc;

      // Stick deflection directly mirrors cursor velocity!
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
      // Phase 1: Resting at Stick, smooth fade-in
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
      // Phase 2: Glide from Stick to Windows Button (heading LEFT)
      const tNorm = (sec - 0.7) / 2.1;
      const motion = interpolateMotion(stickCenter, winCenter, tNorm, -18);
      curX = motion.x;
      curY = motion.y;
      sx = motion.stickTiltX;
      sy = motion.stickTiltY;
      clickedThisSegment = false;
    } else if (sec < 4.1) {
      // Phase 3: At Windows Button, Click sequence
      const tNorm = (sec - 2.8) / 1.3;
      curX = winCenter.x;
      curY = winCenter.y;

      // Soft spring recoil damping as stick returns to deadzone
      const decay = Math.exp(-tNorm * 9);
      sx = decay * Math.sin(tNorm * Math.PI * 4) * 2;
      sy = 0;

      // Click window: from 20% to 65% of hold phase
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
      // Phase 4: Glide from Windows Button to Linux Button (heading RIGHT)
      const tNorm = (sec - 4.1) / 1.6;
      const motion = interpolateMotion(winCenter, linuxCenter, tNorm, 14);
      curX = motion.x;
      curY = motion.y;
      sx = motion.stickTiltX;
      sy = motion.stickTiltY;
      clickedThisSegment = false;
      windows.classList.remove("is-demo-active");
    } else if (sec < 7.0) {
      // Phase 5: At Linux Button, Click sequence
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
      // Phase 6: Return Glide from Linux Button to Stick (heading RIGHT)
      const tNorm = (sec - 7.0) / 1.6;
      const motion = interpolateMotion(linuxCenter, stickCenter, tNorm, -14);
      curX = motion.x;
      curY = motion.y;
      sx = motion.stickTiltX;
      sy = motion.stickTiltY;
      clickedThisSegment = false;
      linux.classList.remove("is-demo-active");
    } else {
      // Phase 7: Settle at Stick, gentle fade out
      const tNorm = (sec - 8.6) / 0.6;
      curX = stickCenter.x;
      curY = stickCenter.y;
      opacity = Math.max(0, 1 - tNorm);
      sx = 0;
      sy = 0;
    }

    // Apply Stick & Left Thumb Transforms
    setStick(sx, sy);

    // Apply Cursor Position (aligned to cursor pointer tip at 3,3)
    cursorContainer.removeAttribute("hidden");
    cursorContainer.style.opacity = opacity.toFixed(3);
    cursorContainer.style.transform = `translate3d(${(curX - 3).toFixed(1)}px, ${(curY - 3).toFixed(1)}px, 0)`;

    // Button A and Right Thumb Press
    buttonA.classList.toggle("is-pressed", clicking);
    cursorContainer.classList.toggle("is-clicking", clicking);

    if (thumbRight) {
      thumbRight.style.transform = clicking
        ? "translate(-12px, -11px) scale(0.95)"
        : "translate(0px, 0px) scale(1)";
    }
  }

  // ---------------------------------------------------------------------------
  // Main Animation Loop
  // ---------------------------------------------------------------------------
  function frame(timestamp) {
    const delta = Math.min(64, timestamp - lastTime);
    lastTime = timestamp;

    const visible = inView && tabVisible && !reducedMotion.matches;

    if (visible) {
      const handled = pollGamepad();
      if (!handled) {
        runShowcase(delta);
      }
    } else {
      cursorContainer.setAttribute("hidden", "");
      resetStick();
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
        resetStick();
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
      resetStick();
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
