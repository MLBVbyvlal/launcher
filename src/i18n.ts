export type Lang = 'en' | 'ru'

const S: Record<Lang, Record<string, string>> = {
  en: {
    // Setup — welcome
    'sw.welcome.title': 'Welcome to MLBV',
    'sw.welcome.sub': 'Minecraft Launcher by vlal',
    'sw.welcome.choose': 'Choose your language to get started',

    // Setup — prefs
    'sw.prefs.title': 'Preferences',
    'sw.prefs.sub': 'You can change these later in Settings',
    'sw.prefs.ram': 'RAM Allocation',
    'sw.prefs.dl': 'Parallel Downloads',
    'sw.prefs.close': 'Hide launcher when game starts',

    // Setup — account
    'sw.acct.title': 'Add an Account',
    'sw.acct.sub': 'Connect your Minecraft account. You can add more later.',
    'sw.acct.ms.label': 'Microsoft',
    'sw.acct.ms.sub': 'Official · Full server access',
    'sw.acct.offline.label': 'Guest',
    'sw.acct.offline.sub': 'Offline · Limited servers',
    'sw.acct.skip': 'Skip for now',

    // Setup — offline warning
    'sw.warn.title': 'Heads up',
    'sw.warn.body': "Minecraft is paid software. An offline account lets you play locally, but you won't be able to join most online servers that require a valid Microsoft account.\n\nFor full multiplayer access, we recommend purchasing Minecraft at minecraft.net.",
    'sw.warn.skip': 'I understand, Skip',
    'sw.warn.buy': 'Buy Minecraft',

    // Setup — nickname
    'sw.nick.title': 'Choose a Username',
    'sw.nick.ph': 'Enter username…',
    'sw.nick.hint': '3–16 characters: letters, numbers and _',
    'sw.nick.err.short': 'At least 3 characters required',
    'sw.nick.err.long': '16 characters maximum',
    'sw.nick.err.chars': 'Only a–z, A–Z, 0–9 and _ allowed',
    'sw.nick.add': 'Add Account',

    // Setup — nick warning
    'sw.nick.warn.title': 'Non-standard username',
    'sw.nick.warn.body': "This username doesn't meet the Minecraft standard (a–z, A–Z, 0–9, _, 3–16 chars). You may not be able to join many online servers.",
    'sw.nick.warn.tips': 'Good examples: Steve · NotchFan99 · pro_player',
    'sw.nick.warn.back': 'Change username',
    'sw.nick.warn.ok': 'Continue anyway',

    // Setup — Java
    'sw.java.title': 'Almost there!',
    'sw.java.sub': 'Downloading missing Java runtimes in the background…',
    'sw.java.done_sub': 'All done — everything is ready.',
    'sw.java.already': 'Already installed',
    'sw.java.downloading': 'Downloading…',
    'sw.java.installing': 'Installing…',
    'sw.java.done': 'Ready',
    'sw.java.error': 'Failed — will retry on launch',
    'sw.java.finish': 'Start Launcher',
    'sw.java.wait': 'Please wait…',

    // Features showcase
    'feat.lb': 'LiquidBounce Built-in',
    'feat.lb_s': 'HvH ready, straight from the launcher',
    'feat.lb_d': 'LiquidBounce is built right in — no extra downloads, no setup. Join HvH private servers and dominate with ESP, killaura, and over 200 modules at your fingertips.',
    'feat.dl': 'Parallel Downloads',
    'feat.dl_s': 'Assets download up to 50× faster than vanilla',
    'feat.dl_d': 'Our multi-threaded download engine fetches assets, libraries, and mods simultaneously across up to 50 threads. Say goodbye to the vanilla launcher\'s slow sequential downloads.',
    'feat.java': 'Auto Java',
    'feat.java_s': 'Right version, every time — automatically',
    'feat.java_d': 'Java 8, 17, 21, 25 — MLBV automatically detects which version each Minecraft release needs and installs it silently in the background. You never touch a JDK installer again.',
    'feat.offline': 'Offline Mode',
    'feat.offline_s': 'Play without a Microsoft account',
    'feat.offline_d': 'No license? No Microsoft account? No problem. Create a guest profile and play on any offline-mode server or local world instantly — no sign-in required.',
    'feat.instances': 'Instance Manager',
    'feat.instances_d': 'Create unlimited instances, each with its own mods, worlds, resource packs, and settings. Run Minecraft 1.7.10 and 1.21 side by side without any conflicts.',
    'feat.custom': 'Full Control',
    'feat.custom_s': 'RAM, resolution, downloads — all yours',
    'feat.custom_d': 'Tune RAM allocation, control download threads, set custom Java flags, choose resolution — every setting is exposed and easy to change. Your launcher, your rules.',

    // Buttons
    'btn.next': 'Next',
    'btn.back': 'Back',
    'btn.skip': 'Skip',

    // Main UI
    'tab.mc': 'Minecraft',
    'tab.lb': 'LiquidBounce',
    'play': 'Play',
    'play.lb': 'Launch LiquidBounce',
    'stop': 'Stop',
    'no_account': 'Select an account',
    'no_instance': 'Create an instance',
    'add_account': '+ Add Account',
    'new_instance': '+ New Instance',

    // "Other tab running"
    'running.lb': 'LiquidBounce is running — switch to LB tab to stop',
    'running.mc': 'Minecraft is running — switch to MC tab to stop',
    'busy': 'Another game is launching — please wait',

    // Settings
    'settings.title': 'Settings',
    'settings.game_dir': 'Game Directory',
    'settings.ram': 'RAM',
    'settings.concurrent': 'Parallel Downloads',
    'settings.java': 'Java',
    'settings.about': 'About',
    'settings.close_on_launch': 'Hide launcher when game starts',
    'settings.reset_setup': 'Reset Setup Wizard',
    'settings.reset_setup_hint': 'The wizard will appear again on next launch.',
    'settings.done': 'Done',
    'settings.java_auto': '↓ auto-download',
    'settings.java_found': '✓ found',
    'settings.java_hint': 'Missing versions download automatically on first launch.',
  },

  ru: {
    // Setup — welcome
    'sw.welcome.title': 'Добро пожаловать в MLBV',
    'sw.welcome.sub': 'Minecraft Launcher by vlal',
    'sw.welcome.choose': 'Выберите язык для начала работы',

    // Setup — prefs
    'sw.prefs.title': 'Настройки',
    'sw.prefs.sub': 'Можно изменить позже в меню Настройки',
    'sw.prefs.ram': 'Выделение RAM',
    'sw.prefs.dl': 'Параллельных загрузок',
    'sw.prefs.close': 'Скрывать лаунчер при запуске игры',

    // Setup — account
    'sw.acct.title': 'Добавить аккаунт',
    'sw.acct.sub': 'Подключите аккаунт Minecraft. Можно добавить ещё позже.',
    'sw.acct.ms.label': 'Microsoft',
    'sw.acct.ms.sub': 'Официальный · Полный доступ к серверам',
    'sw.acct.offline.label': 'Гость',
    'sw.acct.offline.sub': 'Оффлайн · Ограниченные серверы',
    'sw.acct.skip': 'Пропустить пока',

    // Setup — offline warning
    'sw.warn.title': 'Внимание',
    'sw.warn.body': 'Minecraft — платная игра. Оффлайн аккаунт позволяет играть локально, но большинство онлайн-серверов требует лицензию Microsoft.\n\nДля полного многопользовательского доступа рекомендуем купить Minecraft на minecraft.net.',
    'sw.warn.skip': 'Я понял, пропустить',
    'sw.warn.buy': 'Купить Minecraft',

    // Setup — nickname
    'sw.nick.title': 'Выберите никнейм',
    'sw.nick.ph': 'Введите никнейм…',
    'sw.nick.hint': '3–16 символов: буквы, цифры и _',
    'sw.nick.err.short': 'Минимум 3 символа',
    'sw.nick.err.long': 'Максимум 16 символов',
    'sw.nick.err.chars': 'Только a–z, A–Z, 0–9 и _ разрешены',
    'sw.nick.add': 'Добавить аккаунт',

    // Setup — nick warning
    'sw.nick.warn.title': 'Нестандартный никнейм',
    'sw.nick.warn.body': 'Этот никнейм не соответствует стандарту Minecraft (a–z, A–Z, 0–9, _, 3–16 символов). Вы можете не попасть на многие онлайн-серверы.',
    'sw.nick.warn.tips': 'Хорошие примеры: Steve · NotchFan99 · pro_player',
    'sw.nick.warn.back': 'Изменить никнейм',
    'sw.nick.warn.ok': 'Всё равно продолжить',

    // Setup — Java
    'sw.java.title': 'Почти готово!',
    'sw.java.sub': 'Загружаем недостающие версии Java в фоне…',
    'sw.java.done_sub': 'Всё готово — можно запускать.',
    'sw.java.already': 'Уже установлена',
    'sw.java.downloading': 'Загрузка…',
    'sw.java.installing': 'Установка…',
    'sw.java.done': 'Готово',
    'sw.java.error': 'Ошибка — повторим при запуске',
    'sw.java.finish': 'Запустить лаунчер',
    'sw.java.wait': 'Подождите…',

    // Features showcase
    'feat.lb': 'LiquidBounce встроен',
    'feat.lb_s': 'HvH прямо из лаунчера',
    'feat.lb_d': 'LiquidBounce уже встроен — никаких лишних загрузок, никакой настройки. Заходи на приватные HvH сервера и доминируй с ESP, килаурой и более 200 модулями.',
    'feat.dl': 'Параллельные загрузки',
    'feat.dl_s': 'Ассеты качаются до 50× быстрее ванили',
    'feat.dl_d': 'Наш движок параллельных загрузок качает ассеты, библиотеки и моды одновременно по 50 потокам. Забудь о медленных последовательных загрузках ванильного лаунчера.',
    'feat.java': 'Авто Java',
    'feat.java_s': 'Нужная версия всегда под рукой',
    'feat.java_d': 'Java 8, 17, 21, 25 — MLBV автоматически определяет, какая версия нужна каждому релизу Minecraft, и тихо устанавливает её. Больше никаких установщиков JDK.',
    'feat.offline': 'Оффлайн режим',
    'feat.offline_s': 'Играй без аккаунта Microsoft',
    'feat.offline_d': 'Нет лицензии? Нет аккаунта Microsoft? Не проблема. Создай гостевой профиль и играй на любом оффлайн-сервере или в локальном мире прямо сейчас — без авторизации.',
    'feat.instances': 'Менеджер инстансов',
    'feat.instances_d': 'Создавай неограниченное количество инстансов, каждый со своими модами, мирами, ресурспаками и настройками. Запускай Minecraft 1.7.10 и 1.21 бок о бок без конфликтов.',
    'feat.custom': 'Полный контроль',
    'feat.custom_s': 'RAM, разрешение, загрузки — всё твоё',
    'feat.custom_d': 'Настрой объём RAM, управляй потоками загрузки, задай кастомные флаги Java, выбери разрешение — каждая настройка доступна и легко меняется. Твой лаунчер, твои правила.',

    // Buttons
    'btn.next': 'Далее',
    'btn.back': 'Назад',
    'btn.skip': 'Пропустить',

    // Main UI
    'tab.mc': 'Minecraft',
    'tab.lb': 'LiquidBounce',
    'play': 'Играть',
    'play.lb': 'Запустить LiquidBounce',
    'stop': 'Остановить',
    'no_account': 'Выберите аккаунт',
    'no_instance': 'Создайте инстанс',
    'add_account': '+ Добавить аккаунт',
    'new_instance': '+ Новый инстанс',

    // "Other tab running"
    'running.lb': 'LiquidBounce запущен — перейди во вкладку LB чтобы остановить',
    'running.mc': 'Minecraft запущен — перейди во вкладку MC чтобы остановить',
    'busy': 'Запуск уже идёт — подождите',

    // Settings
    'settings.title': 'Настройки',
    'settings.game_dir': 'Директория игры',
    'settings.ram': 'RAM',
    'settings.concurrent': 'Параллельных загрузок',
    'settings.java': 'Java',
    'settings.about': 'О лаунчере',
    'settings.close_on_launch': 'Скрывать лаунчер при запуске игры',
    'settings.reset_setup': 'Сбросить мастер настройки',
    'settings.reset_setup_hint': 'При следующем запуске мастер появится снова.',
    'settings.done': 'Готово',
    'settings.java_auto': '↓ автоскачка',
    'settings.java_found': '✓ найдена',
    'settings.java_hint': 'Отсутствующие версии скачаются автоматически при запуске.',
  },
}

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
