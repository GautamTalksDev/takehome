document.querySelectorAll('button.copy').forEach((button) => {
  button.addEventListener('click', () => {
    const source = document.getElementById(button.getAttribute('data-copy'));
    const text = source ? source.textContent : '';
    button.textContent = 'Copied';
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).catch(() => {});
    }
  });
});
