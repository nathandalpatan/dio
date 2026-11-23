import { useEffect, useRef, useState } from 'react';
import { AppGallery, Install, PopupPreview, Questions, SiteFooter, SiteNav, Speech, download } from './shared';

function ContextDemo() {
  const stage = useRef<HTMLDivElement>(null);
  const [inView, setInView] = useState(false);
  const [pageVisible, setPageVisible] = useState(!document.hidden);
  const [reducedMotion, setReducedMotion] = useState(window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false);
  const [paused, setPaused] = useState(false);
  const [cycle, setCycle] = useState(0);

  useEffect(() => {
    const observer = typeof IntersectionObserver === 'undefined' ? null : new IntersectionObserver(([entry]) => setInView(entry.isIntersecting), { threshold: .25 });
    if (observer && stage.current) observer.observe(stage.current);
    else setInView(true);
    const motion = window.matchMedia?.('(prefers-reduced-motion: reduce)');
    const onVisibilityChange = () => setPageVisible(!document.hidden);
    const onMotionChange = () => setReducedMotion(motion?.matches ?? false);
    document.addEventListener('visibilitychange', onVisibilityChange);
    motion?.addEventListener?.('change', onMotionChange);
    return () => {
      observer?.disconnect();
      document.removeEventListener('visibilitychange', onVisibilityChange);
      motion?.removeEventListener?.('change', onMotionChange);
    };
  }, []);

  return <div className="context-demo" data-running={!paused && inView && pageVisible && !reducedMotion} data-reduced-motion={reducedMotion}>
    <div className="context-demo__stage" ref={stage} role="img" aria-label="Animated demonstration: Neo asks Remi for an invite; ephemeral is selected in Remi's reply and DIO explains that it means temporary.">
      <div className="context-demo__animation" key={cycle}>
        <div className="context-demo__chat" aria-hidden="true">
          <div className="context-demo__chat-bar"><span>Direct Message</span><strong>Remi</strong></div>
          <div className="context-demo__messages">
            <div className="context-demo__message"><span className="context-demo__avatar context-demo__avatar--neo">N</span><div><strong>Neo</strong><p>Can you send the event invite again? I can’t find it.</p></div></div>
            <div className="context-demo__message"><span className="context-demo__avatar context-demo__avatar--remi">R</span><div><strong>Remi</strong><p>Sure. The first link was <span className="context-demo__selected">ephemeral</span>.</p></div></div>
          </div>
          <div className="context-demo__composer">Message Remi</div>
        </div>
        <div className="site-popup context-demo__popup" aria-hidden="true">
          <div className="site-popup__bar"><span>←</span><strong>DIO</strong><span>×</span></div>
          <div className="site-popup__body">
            <div className="site-popup__word"><h3>ephemeral</h3><span className="context-demo__star">☆</span></div>
            <div className="site-popup__ipa"><em>adjective</em><span>US</span><span>/əˈfɛ.mə.rəl/</span><span>◖))</span></div>
            <div className="site-popup__tabs"><span className="context-demo__active-tab">Definition</span><span>Examples</span><span>Synonyms</span></div>
            <div className="site-popup__content"><b>adjective</b><p>Something that is ephemeral is only temporary, it does not last long.</p><small>Example</small><p>Sure. The first link was ephemeral.</p></div>
          </div>
          <div className="site-popup__footer"><span>Kokoro Heart · US English</span><span>Expand ↗</span></div>
        </div>
      </div>
    </div>
    {!reducedMotion && <div className="context-demo__controls"><button onClick={() => setPaused(value => !value)}>{paused ? 'Play demo' : 'Pause demo'}</button><button onClick={() => { setCycle(value => value + 1); setPaused(false); }}>Replay demo</button></div>}
  </div>;
}

export function WebsiteB() { return <div className="site site-b" id="top"><SiteNav concept="Word in Context" quietMenu/><main>
  <section className="site-hero"><div className="site-wrap"><div className="site-hero__copy"><span className="eyebrow">Dictionary for Windows</span><h1>A dictionary beside your work</h1><p>Select a word in a Windows app and press Ctrl + Alt + D. DIO opens its meaning where you are, with 152,000 words offline.</p><a className="site-button" href={download}>Download for Windows</a></div><div className="site-hero__product context-scene"><div className="context-scene__document"><span>Reading in another app</span><p>The <mark>collisions</mark> were difficult to predict.</p></div><PopupPreview/></div></div></section>
  <section className="site-section how-section" id="how"><div className="site-wrap"><div className="section-intro"><span className="eyebrow">The lookup</span><h2>One selection, one shortcut</h2><p>Select a word, press Ctrl + Alt + D, and read beside your source. This animated demonstration uses the names Neo and Remi.</p></div><ContextDemo/></div></section>
  <section className="site-section apps-section" id="apps"><div className="site-wrap split-section"><div className="section-intro"><h2>Made for reading across Windows</h2><p>Real captures show the popup in the apps where people read.</p></div><AppGallery/></div></section>
  <Speech/><Install/><Questions/>
  </main><SiteFooter/></div>; }
