export const insertCustomFontStyle = (customFont) => {
  if (typeof document === 'undefined') return
  const head = document.querySelector('head')
  if (!head) return

  const existing = head.querySelector('#__CUSTOM_FONT__')
  if (!customFont) {
    existing?.remove()
    return
  }

  const escapedFont = String(customFont)
    .replaceAll('\\', '\\\\')
    .replaceAll('"', '\\"')
  const css = `
    @font-face {
      font-family: SourceHanSansCN-Bold;
      font-weight: 700;
      src: local("${escapedFont}");
    }
  `

  if (existing) {
    existing.textContent = css
    return
  }

  const style = document.createElement('style')
  style.setAttribute('id', '__CUSTOM_FONT__')
  style.textContent = css
  head.appendChild(style)
}
