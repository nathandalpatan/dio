import { useState } from 'react';
import { Icon, RelatedLinks, SettingsPanel, Status, type VariantProps } from './shared';

export default function WordStage({ model, actions }: VariantProps) {
  const [open, setOpen] = useState<'examples' | 'related' | 'source' | null>(null);
  const entry = model.entry;
  const toggle = (name: typeof open) => setOpen(current => current === name ? null : name);

  return <section className="lookup lookup--stage" aria-label="Word Stage lookup">
    <header className="stage-header"><div className="stage-header__left"><button className="icon-target" onClick={actions.back} disabled={!model.canBack} aria-label="Back to previous word"><Icon name="back" /></button><span>DIO</span></div><button className="icon-target" onClick={actions.close} aria-label="Close lookup"><Icon name="close" /></button></header>
    <div className="lookup-scroll">
      {model.status !== 'ready' || !entry ? <Status model={model} /> : <>
        <div className="stage-word"><span className="stage-eyebrow">WORD / {entry.part.toUpperCase()}</span><h1>{entry.word}</h1><div className="stage-ipa"><span>US {entry.ipa}</span><button className="stage-sound" onClick={actions.pronounce} aria-label="Play pronunciation"><Icon name="speaker" size={18} /></button></div></div>
        <div className="stage-definition"><div className="stage-definition__label">Meaning 01</div><p>{entry.primary}</p></div>
        {model.expanded ? <div className="stage-more">
          {entry.meanings.length > 0 && <div><h2>More meanings</h2><ol>{entry.meanings.map(meaning => <li key={meaning}>{meaning}</li>)}</ol></div>}
          {entry.examples.length > 0 && <div><h2>In use</h2>{entry.examples.map(example => <blockquote key={example}>{example}</blockquote>)}</div>}
          <div><h2>Related words</h2><RelatedLinks entry={entry} lookup={actions.lookup} /></div>
          {entry.origin && <div><h2>Origin</h2><p>{entry.origin}</p></div>}
          <p className="entry-source">Source: {entry.source}</p>
        </div> : <div className="stage-explore">
          <button onClick={() => toggle('examples')} aria-expanded={open === 'examples'}>In use <span>{entry.examples.length.toString().padStart(2, '0')} <Icon name="chevron" size={14} /></span></button>
          {open === 'examples' && <div className="stage-explore__body">{entry.examples.map(example => <blockquote key={example}>{example}</blockquote>)}</div>}
          <button onClick={() => toggle('related')} aria-expanded={open === 'related'}>Related words <span>{entry.related.length.toString().padStart(2, '0')} <Icon name="chevron" size={14} /></span></button>
          {open === 'related' && <div className="stage-explore__body"><RelatedLinks entry={entry} lookup={actions.lookup} />{entry.origin && <p>{entry.origin}</p>}</div>}
          <button onClick={() => toggle('source')} aria-expanded={open === 'source'}>Source <span><Icon name="chevron" size={14} /></span></button>
          {open === 'source' && <p className="stage-explore__body entry-source">{entry.source}</p>}
        </div>}
      </>}
    </div>
    <footer className="stage-footer">
      <button onClick={actions.save} disabled={!entry || model.status !== 'ready'} aria-pressed={model.saved}><Icon name="star" size={16} filled={model.saved} />{model.saved ? 'Saved' : 'Save'}</button>
      <button onClick={model.speaking ? actions.stop : actions.read} disabled={!entry || model.status !== 'ready'}><Icon name={model.speaking ? 'stop' : 'play'} size={15} />{model.speaking ? 'Stop' : 'Read'}</button>
      <button onClick={actions.settings} aria-label="Speech settings"><Icon name="settings" size={17} /></button>
      <button onClick={actions.expand} aria-expanded={model.expanded} aria-label={model.expanded ? 'Collapse entry' : 'Expand entry'}><Icon name={model.expanded ? 'collapse' : 'expand'} size={17} /></button>
    </footer>
    <SettingsPanel model={model} actions={actions} />
  </section>;
}
