export const defaultUiLocale = 'pt-BR' as const;

/**
 * DeadVox currently ships its interface in Brazilian Portuguese. We still
 * inspect the browser/Windows language list so the first-run choice is
 * explicit and ready for future interface languages; Portuguese is also the
 * safe fallback for an incomplete or unavailable system locale.
 */
export function detectUiLocale(languages?: readonly string[]): typeof defaultUiLocale {
  const candidates = languages ?? (typeof navigator === 'undefined'
    ? []
    : navigator.languages?.length ? navigator.languages : [navigator.language]);
  return candidates.some(language => /^pt(?:-|$)/i.test(language)) ? 'pt-BR' : defaultUiLocale;
}

export function applyDetectedUiLocale(): typeof defaultUiLocale {
  const locale = detectUiLocale();
  document.documentElement.lang = locale;
  return locale;
}
