import { AppGallery, Install, PopupPreview, Questions, SiteFooter, SiteNav, Speech, Walkthrough, download } from './shared';

export function WebsiteA() { return <div className="site site-a" id="top"><SiteNav concept="Quiet Reference"/><main>
  <section className="site-hero"><div className="site-wrap site-hero__grid"><div className="site-hero__copy"><span className="eyebrow">Dictionary for Windows</span><h1>A pop-up dictionary for Windows</h1><p>Select a word, press Ctrl + Alt + D, and read its meaning beside your work. More than 152,000 words are available offline.</p><a className="site-button" href={download}>Download for Windows</a></div><div className="site-hero__product"><PopupPreview/></div></div></section>
  <section className="site-section how-section" id="how"><div className="site-wrap split-section"><div className="section-intro"><h2>From selected word to answer</h2><p>One shortcut keeps the definition close to what you are reading.</p></div><Walkthrough/></div></section>
  <section className="site-section apps-section" id="apps"><div className="site-wrap split-section"><div className="section-intro"><h2>Works in the apps you read in</h2><p>See DIO in browsers, PDF readers, terminals, and game stores.</p></div><AppGallery/></div></section>
  <Speech/><Install/><Questions/>
  </main><SiteFooter/></div>; }
