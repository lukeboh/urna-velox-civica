// Páginas de texto (/termos, /privacidade): o mesmo tema que a pessoa escolheu na página da apuração (lido do navegador, nada sai dele).
try { const t = localStorage.getItem('uv-tema'); if (t) document.documentElement.dataset.tema = t; } catch { /* sem armazenamento */ }
