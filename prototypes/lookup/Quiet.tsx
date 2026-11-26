import { useEffect, useState } from 'react';
import { Icon, SettingsPanel, Status, type VariantProps } from './shared';

type Section = 'definition' | 'examples' | 'synonyms';

export default function Quiet({ model, actions }: VariantProps) {
  const [tab, setTab] = useState<Section>('definition');
  const entry = model.entry;
  const tabs: Section[] = ['definition'];
  if (entry?.examples.length) tabs.push('examples');
  if (entry?.synonyms?.length) tabs.push('synonyms');
  const active = tabs.includes(tab) ? tab : 'definition';
  const synonyms = entry?.synonyms?.map(word => <button key={word} className="text-link" onClick={() => actions.lookup(word)}>{word}</button>);

  useEffect(() => setTab('definition'), [entry?.word]);

  return <section className="lookup lookup--quiet" aria-label="Quiet Reference lookup">
    <header className="quiet-header">
      <div className="quiet-header__nav"><button className="icon-target" onClick={actions.back} aria-label={model.canBack ? 'Back to previous word' : 'Back to reading'}><Icon name="back" /></button>{model.canForward && <button className="icon-target" onClick={actions.forward} aria-label="Forward to next word"><Icon name="forward" /></button>}</div>
      <span className="quiet-brand">DIO</span>
      <button className="icon-target" onClick={actions.close} aria-label="Close lookup"><Icon name="close" /></button>
    </header>
    <div className="lookup-scroll">
      {model.status !== 'ready' || !entry ? <Status model={model} /> : <>
        <div className="quiet-wordline"><h1>{entry.word}</h1><button className="icon-target" onClick={actions.save} aria-label={model.saved ? 'Remove saved word' : 'Save word'} aria-pressed={model.saved}><Icon name="star" filled={model.saved} /></button></div>
        <div className="quiet-phonetic"><span>{entry.part}</span><b>US</b><span>{entry.ipa}</span><button className="icon-target" onClick={actions.pronounce} aria-label="Play pronunciation"><Icon name="speaker" size={17} /></button></div>
        {model.expanded ? <div className="quiet-expanded">
          <h2>Definition</h2><p className="quiet-definition">{entry.primary}</p>
          {entry.meanings.length > 0 && <><h2>More meanings</h2><ol>{entry.meanings.map(meaning => <li key={meaning}>{meaning}</li>)}</ol></>}
          {entry.examples.length > 0 && <><h2>Examples</h2>{entry.examples.map(example => <blockquote key={example}>{example}</blockquote>)}</>}
          {!!entry.synonyms?.length && <><h2>Synonyms</h2><div className="related-links">{synonyms}</div></>}
          {entry.origin && <><h2>Origin</h2><p>{entry.origin}</p></>}
          <p className="entry-source">Source: {entry.source}</p>
        </div> : <>
          <div className="quiet-tabs" role="tablist" aria-label="Entry sections" onKeyDown={event => {
            if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
            event.preventDefault();
            const next = tabs[(tabs.indexOf(active) + (event.key === 'ArrowRight' ? 1 : tabs.length - 1)) % tabs.length];
            setTab(next);
            (event.currentTarget.querySelector(`[data-tab="${next}"]`) as HTMLButtonElement)?.focus();
          }}>
            {tabs.map(item => <button key={item} type="button" role="tab" data-tab={item} aria-selected={active === item} tabIndex={active === item ? 0 : -1} onClick={() => setTab(item)}>{item[0].toUpperCase() + item.slice(1)}</button>)}
          </div>
          <div className="quiet-tabpanel" role="tabpanel" aria-label={active}>
            {active === 'definition' ? <><span className="quiet-part">{entry.part}</span><p className="quiet-definition">{entry.primary}</p>{entry.examples[0] && <div className="quiet-example"><span>Example</span><p>{entry.examples[0]}</p></div>}</> : active === 'examples' ? entry.examples.map(example => <blockquote key={example}>{example}</blockquote>) : <div className="related-links">{synonyms}</div>}
          </div>
        </>}
      </>}
    </div>
    <footer className="quiet-footer">
      <span className="quiet-voice">Browser speech preview</span>
      {model.speaking ? <button className="icon-target" onClick={actions.stop} aria-label="Stop speech"><Icon name="stop" size={16} /></button> : <button className="icon-target" onClick={actions.read} disabled={!entry || model.status !== 'ready'} aria-label="Read definition aloud"><Icon name="play" size={16} /></button>}
      <button className="icon-target" onClick={actions.settings} aria-label="Speech settings"><Icon name="settings" size={17} /></button>
      <button className="quiet-expand" onClick={actions.expand} aria-expanded={model.expanded}>{model.expanded ? 'Collapse' : 'Expand'} <Icon name={model.expanded ? 'collapse' : 'expand'} size={15} /></button>
    </footer>
    <SettingsPanel model={model} actions={actions} />
  </section>;
}
