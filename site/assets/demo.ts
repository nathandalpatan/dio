const demo = document.querySelector<HTMLElement>('[data-demo]');

if (demo) {
  const scenes = [...demo.querySelectorAll<HTMLElement>('[data-demo-scene]')];
  const copy = [...demo.querySelectorAll<HTMLElement>('[data-demo-copy]')];
  const play = demo.querySelector<HTMLButtonElement>('[data-demo-play]')!;
  const previous = demo.querySelector<HTMLButtonElement>('[data-demo-prev]')!;
  const next = demo.querySelector<HTMLButtonElement>('[data-demo-next]')!;
  const status = demo.querySelector<HTMLElement>('[data-demo-status]')!;
  const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
  let step = 0;
  let timer: number | undefined;
  let playing = false;
  let entrance: Animation | undefined;

  function render(autoAdvance = false) {
    entrance?.cancel();
    entrance = undefined;
    scenes.forEach((scene, index) => scene.classList.toggle('is-active', index === step));
    copy.forEach((item, index) => item.classList.toggle('is-active', index === step));
    if (autoAdvance && !reducedMotion.matches && document.documentElement.dataset.motion !== 'off') {
      const motion = getComputedStyle(document.documentElement);
      entrance = scenes[step].animate([{ opacity: 0 }, { opacity: 1 }], {
        duration: parseFloat(motion.getPropertyValue('--m-panel')) || 200,
        easing: motion.getPropertyValue('--m-ease').trim() || 'cubic-bezier(.2, .8, .2, 1)',
      });
    }
    status.textContent = `${step + 1} / ${scenes.length}`;
    previous.disabled = step === 0;
    next.disabled = step === scenes.length - 1;
    const action = playing ? 'Pause' : step === scenes.length - 1 ? 'Replay' : 'Play';
    play.textContent = action;
    play.setAttribute('aria-label', `${action} walkthrough`);
  }

  function stop() {
    window.clearTimeout(timer);
    playing = false;
    render();
  }

  function advance() {
    if (step === scenes.length - 1) {
      stop();
      return;
    }
    step += 1;
    playing = step !== scenes.length - 1;
    render(true);
    if (playing) timer = window.setTimeout(advance, 5000);
  }

  play.addEventListener('click', () => {
    if (playing) {
      stop();
      return;
    }
    if (step === scenes.length - 1) step = 0;
    playing = true;
    render();
    timer = window.setTimeout(advance, 5000);
  });

  previous.addEventListener('click', () => {
    stop();
    step = Math.max(0, step - 1);
    render();
  });

  next.addEventListener('click', () => {
    stop();
    step = Math.min(scenes.length - 1, step + 1);
    render();
  });

  reducedMotion.addEventListener('change', () => {
    if (reducedMotion.matches) stop();
    play.hidden = reducedMotion.matches;
  });
  document.addEventListener('visibilitychange', () => { if (document.hidden) stop(); });
  play.hidden = reducedMotion.matches;
  render();
}
