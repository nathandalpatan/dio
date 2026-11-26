import { useState } from 'react';
import { Icon, RelatedLinks, SettingsPanel, Status, type VariantProps } from './shared';

const tabs = ['meanings', 'related'] as const;

export default function FastLookup({ model, actions }: VariantProps) {
  const [detail, setDetail] = useState<(typeof tabs)[number]>('meanings');
  const entry = model.entry;

  return <section className="lookup lookup--fast" aria-label="Fast Lookup reference panel">
    <header className="fast-header"><button className="fast-back" onClick={actions.back} disabled={!model.canBack}><Icon name="back" size={16} />Back</button><strong>DIO</strong><div className="fast-header__actions">{model.status !== 'ready' && <button className="icon-target" onClick={actions.settings} aria-label="Voice settings" aria-expanded={model.settingsOpen}><Icon name="settings" size={16} /></button>}<button className="icon-target" onClick={actions.close} aria-label="Close lookup"><Icon name="close" /></button></div></header>
    <div className="lookup-scroll">
      {model.status !== 'ready' || !entry ? <Status model={model} /> : <>
        <div className="fast-heading"><div className="fast-wordline"><h1>{entry.word}</h1><button className="icon-target fast-save" onClick={actions.save} aria-label={model.saved ? 'Remove saved word' : 'Save word'} aria-pressed={model.saved}><Icon name="star" size={19} filled={model.saved} /></button></div><div className="fast-meta"><span>{entry.part}</span><span>US {entry.ipa}</span><div className="fast-audio"><button className="icon-target fast-pronounce" onClick={actions.pronounce} aria-label="Play pronunciation"><Icon name="speaker" size={18} /></button><button className="fast-voice" onClick={actions.settings} aria-expanded={model.settingsOpen}><Icon name="settings" size={15} />Voice</button></div></div></div>
        <div className="fast-primary"><h2>Definition</h2><p>{entry.primary}</p>{entry.examples[0] && <blockquote>{entry.examples[0]}</blockquote>}</div>
        {model.expanded ? <div className="fast-expanded">
          <div><h2>More meanings</h2>{entry.meanings.length ? <ol>{entry.meanings.map(meaning => <li key={meaning}>{meaning}</li>)}</ol> : <p className="fast-muted">No additional meanings.</p>}{entry.examples.slice(1).map(example => <blockquote key={example}>{example}</blockquote>)}</div>
          <div><h2>Related</h2><RelatedLinks entry={entry} lookup={actions.lookup} />{entry.origin && <><h2>Origin</h2><p>{entry.origin}</p></>}<h2>Source</h2><p className="entry-source">{entry.source}</p></div>
        </div> : <>
          <div className="fast-switch" role="tablist" aria-label="Entry sections" onKeyDown={event => {
            if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
            event.preventDefault();
            const next = tabs[(tabs.indexOf(detail) + (event.key === 'ArrowRight' ? 1 : tabs.length - 1)) % tabs.length];
            setDetail(next);
            (event.currentTarget.querySelector(`[data-tab="${next}"]`) as HTMLButtonElement)?.focus();
          }}>{tabs.map(item => <button key={item} type="button" role="tab" data-tab={item} aria-selected={detail === item} tabIndex={detail === item ? 0 : -1} onClick={() => setDetail(item)}>{item === 'meanings' ? 'More meanings' : 'Related & source'}</button>)}</div>
          <div className="fast-details" role="tabpanel" aria-label={detail}>
            {detail === 'meanings' ? entry.meanings.length ? <ol>{entry.meanings.map(meaning => <li key={meaning}>{meaning}</li>)}</ol> : <p className="fast-muted">No additional meanings.</p> : <><RelatedLinks entry={entry} lookup={actions.lookup} />{entry.origin && <p>{entry.origin}</p>}<p className="entry-source">Source: {entry.source}</p></>}
          </div>
        </>}
      </>}
    </div>
    <footer className="fast-footer">
      <button onClick={model.speaking ? actions.stop : actions.read} disabled={!entry || model.status !== 'ready'}><Icon name={model.speaking ? 'stop' : 'play'} size={16} />{model.speaking ? 'Stop reading' : 'Read definition'}</button>
      <button onClick={actions.expand} aria-expanded={model.expanded}><Icon name={model.expanded ? 'collapse' : 'expand'} size={16} />{model.expanded ? 'Collapse' : 'Expand'}</button>
    </footer>
    <SettingsPanel model={model} actions={actions} />
  </section>;
}
