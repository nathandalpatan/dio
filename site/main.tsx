import { createRoot } from 'react-dom/client';
import { WebsiteB } from '../prototypes/design/website/WebsiteB';
import '@fontsource/inter/latin-400.css';
import '@fontsource/inter/latin-500.css';
import '@fontsource/inter/latin-600.css';
import '@fontsource/source-serif-4/latin-400.css';
import '@fontsource/source-serif-4/latin-600.css';
import '@fontsource/source-serif-4/latin-700.css';
import '../prototypes/design/style.css';
import './production.css';

createRoot(document.getElementById('root')!).render(<WebsiteB />);
