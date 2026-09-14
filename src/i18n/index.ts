import en from './en'
import ru from './ru'

export type Lang = 'en' | 'ru'

const S: Record<Lang, Record<string, string>> = { en, ru }

export function useT(lang: Lang) {
  return (key: string): string => S[lang]?.[key] ?? S.en[key] ?? key
}

export function getLang(): Lang {
  const v = localStorage.getItem('mlbv_lang')
  return v === 'ru' ? 'ru' : 'en'
}

export function setLang(lang: Lang) {
  localStorage.setItem('mlbv_lang', lang)
}
