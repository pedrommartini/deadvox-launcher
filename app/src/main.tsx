import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { applyDetectedUiLocale } from './locale';
import './style.css';

const savedTheme = localStorage.getItem('deadvox-theme');
document.documentElement.dataset.theme = savedTheme === 'light' ? 'light' : 'dark';
applyDetectedUiLocale();

ReactDOM.createRoot(document.getElementById('root')!).render(<React.StrictMode><App /></React.StrictMode>);
