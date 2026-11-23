import React from 'react';
import { createRoot } from 'react-dom/client';
import { Picker, useVariant } from './Picker';
import { WebsiteA } from './website/WebsiteA';
import { WebsiteB } from './website/WebsiteB';
import { WebsiteC } from './website/WebsiteC';
import { SettingsA } from './settings/SettingsA';
import { SettingsB } from './settings/SettingsB';
import { SettingsC } from './settings/SettingsC';
import '@fontsource/inter/400.css';
import '@fontsource/inter/500.css';
import '@fontsource/inter/600.css';
import '@fontsource/source-serif-4/400.css';
import '@fontsource/source-serif-4/600.css';
import '@fontsource/source-serif-4/700.css';
import './style.css';

const websites = [WebsiteA, WebsiteB, WebsiteC];
const settings = [SettingsA, SettingsB, SettingsC];
const isSettings = location.pathname.includes('/settings/');

function App() {
  const { variant, setVariant, replay, setReplay } = useVariant();
  const Concept = (isSettings ? settings : websites)[variant];
  const other = isSettings ? '../website/' : '../settings/';
  return <>
    <div className="prototype-shell"><span>DIO / Design exploration</span><a href={`${other}?v=${variant + 1}`}>View {isSettings ? 'website' : 'Settings'} concept ↗</a></div>
    <Concept key={`${variant}-${replay}`} />
    {!new URLSearchParams(location.search).has('capture') && <Picker variant={variant} setVariant={setVariant} setReplay={setReplay}/>}
  </>;
}

createRoot(document.getElementById('prototype-root')!).render(<React.StrictMode><App/></React.StrictMode>);
