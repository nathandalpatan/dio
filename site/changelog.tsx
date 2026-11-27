import React, { useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import changelog from '../CHANGELOG.md?raw';
import { anchor, parseChangelog, type Release } from '../tools/changelog-parser.mjs';
import { SiteFooter, SiteNav } from '../prototypes/design/website/shared';
import '@fontsource/inter/latin-400.css';
import '@fontsource/inter/latin-500.css';
import '@fontsource/inter/latin-600.css';
import '@fontsource/source-serif-4/latin-400.css';
import '@fontsource/source-serif-4/latin-600.css';
import '@fontsource/source-serif-4/latin-700.css';
import '../prototypes/design/style.css';
import './production.css';

/** Bold, code and https links from a changelog line; anything else stays plain text. */
export function inline(text: string) {
  return text.split(/(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\(https:\/\/[^)\s]+\))/g).map((part, index) => {
    if (part.startsWith('**') && part.endsWith('**') && part.length > 4) return <strong key={index}>{part.slice(2, -2)}</strong>;
    if (part.startsWith('`') && part.endsWith('`') && part.length > 2) return <code key={index}>{part.slice(1, -1)}</code>;
    const link = /^\[([^\]]+)\]\((https:\/\/[^)\s]+)\)$/.exec(part);
    if (link) return <a key={index} href={link[2]}>{link[1]}</a>;
    return <React.Fragment key={index}>{part}</React.Fragment>;
  });
}

const formatDate = (date: string) => new Date(`${date}T00:00:00Z`).toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric', timeZone: 'UTC' });

/** Released versions only: an entry still marked Unreleased stays off the public page. */
export function Changelog({ releases }: { releases: Release[] }) {
  const published = releases.filter(release => release.date);
  useEffect(() => {
    // The entries render after the browser's own jump to the fragment, so jump again.
    if (location.hash) document.getElementById(decodeURIComponent(location.hash.slice(1)))?.scrollIntoView();
  }, []);
  return <div className="site site-b" id="top">
    <SiteNav concept="Word in Context" quietMenu home="../"/>
    <main className="changelog">
      <div className="site-wrap changelog-intro">
        <h1>Changelog</h1>
        <p>New features, improvements and fixes in each version of DIO.</p>
      </div>
      {published.map((release, index) => <article className="site-wrap changelog-release" id={anchor(release.version)} key={release.version}>
        <header>
          <h2><a href={`#${anchor(release.version)}`}>Version {release.version}</a></h2>
          {index === 0 && <span className="changelog-latest">Latest</span>}
          <time dateTime={release.date!}>{formatDate(release.date!)}</time>
        </header>
        <p className="changelog-summary">{inline(release.summary)}</p>
        {release.sections.filter(section => section.items.length).map(section => <section key={section.title}>
          <h3>{section.title}</h3>
          <ul>{section.items.map(item => <li key={item}>{inline(item)}</li>)}</ul>
        </section>)}
      </article>)}
    </main>
    <SiteFooter/>
  </div>;
}

const root = document.getElementById('root');
if (root) createRoot(root).render(<Changelog releases={parseChangelog(changelog)}/>);
