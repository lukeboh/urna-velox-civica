// Tema do painel: o mesmo da página de resultados (localStorage "uv-tema": escuro, o padrão, ou claro). Carregado no
// <head> (sem defer) para aplicar a escolha antes do primeiro desenho; o botão #tema alterna.
(function () {
  try { const t = localStorage.getItem('uv-tema'); if (t) document.documentElement.dataset.tema = t; } catch (_) { /* sem armazenamento */ }
  document.addEventListener('DOMContentLoaded', function () {
    const b = document.getElementById('tema');
    if (!b) return;
    b.addEventListener('click', function () {
      const t = document.documentElement.dataset.tema === 'claro' ? 'escuro' : 'claro';
      document.documentElement.dataset.tema = t;
      try { localStorage.setItem('uv-tema', t); } catch (_) { /* sem armazenamento */ }
      window.dispatchEvent(new Event('uv-tema'));
    });
  });
})();
