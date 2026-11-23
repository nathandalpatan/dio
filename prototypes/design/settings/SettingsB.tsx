import { SettingsNav, SettingsPanel, useSettingsModel } from './shared';

export function SettingsB() { const model = useSettingsModel(); return <div className={`settings-stage theme-${model.settings.theme} ${model.settings.reducedMotion ? 'reduced-motion' : ''}`}><div className="settings-window settings-b"><header className="settings-b__head"><div><span>DIO</span><h2>Your reading, your way.</h2></div><p>Settings for how words appear, sound, and stay with you.</p></header><SettingsNav model={model}/><SettingsPanel model={model}/></div></div>; }
