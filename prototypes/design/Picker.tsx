import { useEffect, useLayoutEffect, useRef, useState } from 'react';

const names = ['Quiet', 'Context', 'Direct'];

export function useVariant() {
  const initial = Number(new URLSearchParams(location.search).get('v'));
  const [variant, setVariant] = useState(initial >= 1 && initial <= 3 ? initial - 1 : 0);
  const [replay, setReplay] = useState(0);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      if (event.altKey || event.ctrlKey || event.metaKey || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName) || target.isContentEditable) return;
      if (/^[1-3]$/.test(event.key)) setVariant(Number(event.key) - 1);
      else if (event.key === 'ArrowLeft') setVariant(current => (current + 2) % 3);
      else if (event.key === 'ArrowRight') setVariant(current => (current + 1) % 3);
      else if (event.key.toLowerCase() === 'r') setReplay(current => current + 1);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
  useEffect(() => {
    const url = new URL(location.href);
    url.searchParams.set('v', String(variant + 1));
    history.replaceState(null, '', url);
  }, [variant]);
  return { variant, setVariant, replay, setReplay };
}

export function Picker({ variant, setVariant, setReplay }: { variant: number; setVariant: (value: number) => void; setReplay: (update: (value: number) => number) => void }) {
  const picker = useRef<HTMLElement>(null);
  const [position, setPosition] = useState({ x: 0, width: 0 });
  useLayoutEffect(() => {
    const nav = picker.current;
    const button = nav?.querySelectorAll<HTMLButtonElement>('.proto-picker-item')[variant];
    if (nav && button) setPosition({ x: button.offsetLeft, width: button.offsetWidth });
  }, [variant]);
  useEffect(() => {
    const id = requestAnimationFrame(() => picker.current?.setAttribute('data-ready', ''));
    return () => cancelAnimationFrame(id);
  }, []);
  return <nav className="proto-picker" aria-label="Prototype variants" ref={picker}>
    <span className="proto-picker-highlight" aria-hidden="true" style={{ transform: `translateX(${position.x}px)`, width: position.width }}></span>
    {names.map((name, index) => <button key={name} className="proto-picker-item" data-active={variant === index ? '' : undefined} aria-current={variant === index ? 'true' : undefined} onClick={() => setVariant(index)}>{name}</button>)}
    <span className="proto-picker-divider" aria-hidden="true"></span>
    <button className="proto-picker-item proto-picker-replay" aria-label="Replay animation (R)" onClick={() => setReplay(value => value + 1)}>↻</button>
  </nav>;
}
