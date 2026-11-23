import { SettingsNav, SettingsPanel, useSettingsModel } from './shared';

export function SettingsA() { const model = useSettingsModel(); return <div className={`settings-stage theme-${model.settings.theme} ${model.settings.reducedMotion ? 'reduced-motion' : ''}`}><div className="settings-window settings-a"><aside className="settings-sidebar"><div className="settings-brand">DIO <span>Settings</span></div><SettingsNav model={model}/><small className="settings-sidebar__footer">Dictionary for Windows</small></aside><SettingsPanel model={model}/></div></div>; }
