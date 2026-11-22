const demo = document.querySelector<HTMLElement>('[data-demo]');

if (demo) {
  const scenes = [...demo.querySelectorAll<HTMLElement>('[data-demo-scene]')];
  const play = demo.querySelector<HTMLButtonElement>('[data-demo-play]')!;
  const previous = demo.querySelector<HTMLButtonElement>('[data-demo-prev]')!;
  const next = demo.querySelector<HTMLButtonElement>('[data-demo-next]')!;
  const status = demo.querySelector<HTMLElement>('[data-demo-status]')!;
  const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
  const labels = ['Select a word', 'Open the lookup', 'Expand the entry'];
  let step = 1;
  let timer: number | undefined;
  let playing = false;

  function render() {
    scenes.forEach((scene, index) => scene.classList.toggle('is-active', index === step));
    status.textContent = `${step + 1} / 3 · ${labels[step]}`;
    previous.disabled = step === 0;
    next.disabled = step === scenes.length - 1;
    play.textContent = playing ? 'Pause' : step === scenes.length - 1 ? 'Replay walkthrough' : 'Play walkthrough';
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
    render();
    timer = window.setTimeout(advance, 2200);
  }

  play.addEventListener('click', () => {
    if (playing) {
      stop();
      return;
    }
    step = 0;
    playing = true;
    render();
    timer = window.setTimeout(advance, 2200);
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
  play.hidden = reducedMotion.matches;
  render();
}
