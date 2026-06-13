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
    'sw.java.all_ready': '✓ All Java runtimes ready',
    'sw.java.dl_progress': 'Downloading Java… {0}%',
    'sw.nick.warn.your_nick': 'Your username:',

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
    'sidebar.account': 'Account',
    'sidebar.instances': 'Instances',
    'settings.language': 'Language',
    'sidebar.expand': 'Expand',
    'sidebar.collapse': 'Collapse',

    // "Other tab running"
    'running.lb': 'LiquidBounce is running — switch to LB tab to stop',
    'running.mc': 'Minecraft is running — switch to MC tab to stop',
    'busy': 'Another game is launching — please wait',

    // Settings tabs
    'settings.title': 'Settings',
    'settings.tab.general': 'General',
    'settings.tab.performance': 'Performance',
    'settings.tab.java': 'Java',
    'settings.tab.about': 'About',
    'settings.tab.danger': 'Danger Zone',

    // Settings — general
    'settings.game_dir': 'Game Directory',
    'settings.close_on_launch': 'Hide launcher when game starts',
    'settings.reset_setup': 'Reset Setup Wizard',
    'settings.reset_setup_hint': 'The wizard will appear again on next launch.',
    'settings.done': 'Done',

    // Settings — performance
    'settings.ram': 'RAM',
    'settings.concurrent': 'Parallel Downloads',

    // Settings — java
    'settings.java': 'Java',
    'settings.java_auto': '↓ auto-download',
    'settings.java_found': '✓ found',
    'settings.java_hint': 'Missing versions download automatically on first launch.',

    // Settings — about
    'settings.about': 'About',
    'settings.check_updates': 'Check for updates',
    'settings.checking': 'Checking…',
    'settings.up_to_date': "You're up to date",
    'settings.update_available': 'available',
    'settings.stack': 'Tauri v2 · Rust · React · TypeScript',
    'settings.disclaimer': 'This launcher is not affiliated with, sponsored by, or endorsed by the LiquidBounce team or CCBlueX.',
    'settings.by': 'by vlalikoffc.',

    // Settings — danger
    'settings.danger_title': 'Danger Zone',
    'settings.danger_hint': 'These actions are permanent and cannot be undone.',
    'settings.danger_btn': 'Delete All Data',
    'settings.danger_warn': '⚠ This will permanently delete:',
    'settings.danger_item1': 'All accounts and login tokens',
    'settings.danger_item2': 'All instances and their configurations',
    'settings.danger_item3': 'Java runtimes downloaded by MLBV',
    'settings.danger_item4': 'Downloaded Minecraft versions, assets and libraries',
    'settings.danger_item5': 'All launcher settings (RAM, downloads, preferences)',
    'settings.danger_wait': 'Please wait {0}s before confirming…',
    'settings.danger_confirm_hint': 'You may now confirm the deletion.',
    'settings.cancel': 'Cancel',
    'settings.deleting': 'Deleting…',
    'settings.confirm_delete': 'Confirm Delete',

    // Status hints below play button
    'status.running': 'Running',
    'status.playing_as': 'Playing as',
    'status.playing_ver': 'MC',

    // Instance modal
    'inst.modal.title_mc': 'New Minecraft Instance',
    'inst.modal.title_lb': 'New LiquidBounce Instance',
    'inst.modal.name': 'Instance Name',
    'inst.modal.name_ph': 'My Instance',
    'inst.modal.version': 'Version',
    'inst.modal.branch': 'Branch',
    'inst.modal.cancel': 'Cancel',
    'inst.modal.create': 'Create',
    'inst.modal.filter.release': 'Releases',
    'inst.modal.filter.snapshot': 'Snapshots',
    'inst.modal.filter.old': 'Old',
    'inst.modal.filter.all': 'All',

    // Context menu
    'ctx.rename': 'Rename',
    'ctx.settings': 'Settings',
    'ctx.reinstall': 'Reinstall',
    'ctx.delete': 'Delete',

    // Reinstall modal
    'reinstall.title': 'Reinstall',
    'reinstall.keep': 'Keep worlds & saves',
    'reinstall.keep_desc': 'Deletes mods, configs, and mod loaders. Keeps worlds, screenshots, resource packs and options.txt.',
    'reinstall.wipe': 'Full wipe',
    'reinstall.wipe_desc': 'Deletes the entire instance folder. All worlds, mods, saves and settings are permanently removed.',
    'reinstall.cancel': 'Cancel',
    'reinstall.doing': 'Reinstalling…',
    'reinstall.btn': 'Reinstall',

    // Instance settings modal
    'isettings.ram': 'RAM Override',
    'isettings.ram_hint': 'Leave empty to use global default',
    'isettings.logs': 'Logs',
    'isettings.open_logs': 'Open logs folder',
    'isettings.no_log': 'No log yet — launch the instance first.',
    'isettings.info': 'Instance Info',
    'isettings.type_lb': 'LiquidBounce (Fabric)',
    'isettings.type_mc': 'Vanilla Minecraft',

    // Update modal
    'update.title': 'Update available',
    'update.no_notes': 'No release notes provided.',
    'update.later': 'Later',
    'update.download': 'Download',
    'update.downloading': 'Downloading update…',
    'update.installing': 'Installing…',
    'update.done': 'Updated to',

    // Add account modal
    'acct.title': 'Add Account',
    'acct.offline_label': 'Offline Account',
    'acct.add': 'Add',
    'acct.or': 'or',
    'acct.ms_label': 'Licensed Account',
    'acct.ms_btn': 'Sign in with Microsoft',
    'acct.ms_loading': 'Complete sign-in in the popup…',
    'acct.ms_note_loading': 'Complete login in the browser window that opened',
    'acct.ms_note': 'Requires a purchased Minecraft license',

    // Misc
    'loading': 'Loading…',
    'error.retry': 'Try again',
    'launcher.subtitle': 'Minecraft Launcher by vlalikoffc',
    'dl.paused': '⏸ Paused',
    'dl.resume': '▶ Resume',
    'dl.pause': '⏸ Pause',
    'dl.cancel': '✕ Cancel',

    // Performance warnings
    'perf.warn.low': 'Low thread count — downloads will be slow.',
    'perf.warn.high': 'High thread count — may cause issues on slow connections.',

    // Settings — game dir hint
    'settings.game_dir_hint': 'Shared libs, assets and Java runtimes are stored here. Instance saves are under instances/.',

    // Settings — Customize tab
    'settings.tab.customize': 'Customize',
    'customize.accent': 'Accent Color',
    'customize.accent_tip': 'Changes the highlight color used throughout the launcher — play button, badges, sliders, and more.',
    'customize.reset': 'Reset',
    'customize.apply_lb': 'Apply to LiquidBounce tab as well',
    'customize.apply_lb_tip': 'Not recommended — LiquidBounce uses blue as its brand color. Applying a different accent may look inconsistent with the LB logo.',
    'customize.bg_orbs': 'Background Orb Color',
    'customize.bg_orbs_tip': 'Changes the ambient glow orbs in the background.',

    // Settings — tooltip hints
    'settings.tip.ram': 'Minimum: 512 MB. Recommended: 2–4 GB for most versions. Setting it too high may actually hurt performance if your system doesn\'t have enough free RAM.',
    'settings.tip.concurrent': 'Number of parallel download threads. More = faster downloads, but may overload slow connections or HDDs.',
    'settings.tip.close_on_launch': 'When enabled, the launcher window hides when a game starts and reappears automatically when the game exits.',
    'settings.tip.reset_setup': 'Wipes the setup-done flag so the setup wizard runs again on next launch. Your accounts and instances are not affected.',

    // Instance errors
    'error.prefix': 'Error:',
    'inst.no_versions': 'No versions available.',
    'inst.name_taken': 'An instance with this name already exists.',
    'isettings.copy_log': 'Copy latest.log',
    'isettings.copied': 'Copied!',
    'reinstall.choose': 'Choose what to keep when reinstalling this instance.',

    // Console toggle
    'settings.console': 'Console Window',
    'settings.console_hint': 'Show live log window when game starts',

    // Mod loader picker
    'inst.loader.title': 'Mod Loader',
    'inst.loader.next': 'Next',
    'inst.loader.vanilla': 'Vanilla',
    'inst.loader.fabric': 'Fabric',
    'inst.loader.vanilla_desc': 'No mods, pure Minecraft',
    'inst.loader.fabric_desc': 'Includes Fabric API from Modrinth',
    'inst.loader.soon': 'Coming soon',

    // Console window
    'console.title': 'Console',
    'console.copy': 'Copy Log',
    'console.open_folder': 'Open Log Folder',
    'console.empty': 'Waiting for game output…',
    'console.lines': 'lines',
    'console.clear': 'Clear',
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
    'sw.java.all_ready': '✓ Все среды Java готовы',
    'sw.java.dl_progress': 'Загрузка Java… {0}%',
    'sw.nick.warn.your_nick': 'Ваш ник:',

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
    'sidebar.account': 'Аккаунт',
    'sidebar.instances': 'Инстансы',
    'settings.language': 'Язык',
    'sidebar.expand': 'Развернуть',
    'sidebar.collapse': 'Свернуть',

    // "Other tab running"
    'running.lb': 'LiquidBounce запущен — перейди во вкладку LB чтобы остановить',
    'running.mc': 'Minecraft запущен — перейди во вкладку MC чтобы остановить',
    'busy': 'Запуск уже идёт — подождите',

    // Settings tabs
    'settings.title': 'Настройки',
    'settings.tab.general': 'Основное',
    'settings.tab.performance': 'Производительность',
    'settings.tab.java': 'Java',
    'settings.tab.about': 'О лаунчере',
    'settings.tab.danger': 'Опасная зона',

    // Settings — general
    'settings.game_dir': 'Директория игры',
    'settings.close_on_launch': 'Скрывать лаунчер при запуске игры',
    'settings.reset_setup': 'Сбросить мастер настройки',
    'settings.reset_setup_hint': 'При следующем запуске мастер появится снова.',
    'settings.done': 'Готово',

    // Settings — performance
    'settings.ram': 'RAM',
    'settings.concurrent': 'Параллельных загрузок',

    // Settings — java
    'settings.java': 'Java',
    'settings.java_auto': '↓ автоскачка',
    'settings.java_found': '✓ найдена',
    'settings.java_hint': 'Отсутствующие версии скачаются автоматически при запуске.',

    // Settings — about
    'settings.about': 'О лаунчере',
    'settings.check_updates': 'Проверить обновления',
    'settings.checking': 'Проверяем…',
    'settings.up_to_date': 'Обновлений нет',
    'settings.update_available': 'доступно',
    'settings.stack': 'Tauri v2 · Rust · React · TypeScript',
    'settings.disclaimer': 'Этот лаунчер не является аффилированным, спонсируемым или одобренным командой LiquidBounce или CCBlueX.',
    'settings.by': 'by vlalikoffc.',

    // Settings — danger
    'settings.danger_title': 'Опасная зона',
    'settings.danger_hint': 'Эти действия необратимы.',
    'settings.danger_btn': 'Удалить все данные',
    'settings.danger_warn': '⚠ Будет удалено навсегда:',
    'settings.danger_item1': 'Все аккаунты и токены авторизации',
    'settings.danger_item2': 'Все инстансы и их настройки',
    'settings.danger_item3': 'Java, скачанная лаунчером',
    'settings.danger_item4': 'Версии Minecraft, ассеты и библиотеки',
    'settings.danger_item5': 'Все настройки лаунчера (RAM, загрузки, параметры)',
    'settings.danger_wait': 'Подождите {0}с перед подтверждением…',
    'settings.danger_confirm_hint': 'Теперь можно подтвердить удаление.',
    'settings.cancel': 'Отмена',
    'settings.deleting': 'Удаляем…',
    'settings.confirm_delete': 'Подтвердить удаление',

    // Status hints below play button
    'status.running': 'Запущено',
    'status.playing_as': 'Играет',
    'status.playing_ver': 'MC',

    // Instance modal
    'inst.modal.title_mc': 'Новый инстанс Minecraft',
    'inst.modal.title_lb': 'Новый инстанс LiquidBounce',
    'inst.modal.name': 'Название инстанса',
    'inst.modal.name_ph': 'Мой инстанс',
    'inst.modal.version': 'Версия',
    'inst.modal.branch': 'Ветка',
    'inst.modal.cancel': 'Отмена',
    'inst.modal.create': 'Создать',
    'inst.modal.filter.release': 'Релизы',
    'inst.modal.filter.snapshot': 'Снапшоты',
    'inst.modal.filter.old': 'Старые',
    'inst.modal.filter.all': 'Все',

    // Context menu
    'ctx.rename': 'Переименовать',
    'ctx.settings': 'Настройки',
    'ctx.reinstall': 'Переустановить',
    'ctx.delete': 'Удалить',

    // Reinstall modal
    'reinstall.title': 'Переустановить',
    'reinstall.keep': 'Сохранить миры',
    'reinstall.keep_desc': 'Удаляет моды, конфиги и загрузчики. Сохраняет миры, скриншоты, ресурспаки и options.txt.',
    'reinstall.wipe': 'Полная очистка',
    'reinstall.wipe_desc': 'Удаляет папку инстанса целиком. Все миры, моды, сохранения и настройки будут удалены безвозвратно.',
    'reinstall.cancel': 'Отмена',
    'reinstall.doing': 'Переустанавливаем…',
    'reinstall.btn': 'Переустановить',

    // Instance settings modal
    'isettings.ram': 'Переопределение RAM',
    'isettings.ram_hint': 'Оставьте пустым для глобального значения',
    'isettings.logs': 'Логи',
    'isettings.open_logs': 'Открыть папку с логами',
    'isettings.no_log': 'Логов пока нет — сначала запустите инстанс.',
    'isettings.info': 'Информация об инстансе',
    'isettings.type_lb': 'LiquidBounce (Fabric)',
    'isettings.type_mc': 'Vanilla Minecraft',

    // Update modal
    'update.title': 'Доступно обновление',
    'update.no_notes': 'Описание обновления не предоставлено.',
    'update.later': 'Позже',
    'update.download': 'Скачать',
    'update.downloading': 'Загрузка обновления…',
    'update.installing': 'Установка…',
    'update.done': 'Обновлено до',

    // Add account modal
    'acct.title': 'Добавить аккаунт',
    'acct.offline_label': 'Оффлайн аккаунт',
    'acct.add': 'Добавить',
    'acct.or': 'или',
    'acct.ms_label': 'Лицензионный аккаунт',
    'acct.ms_btn': 'Войти через Microsoft',
    'acct.ms_loading': 'Завершите вход в открывшемся окне…',
    'acct.ms_note_loading': 'Завершите вход в открывшемся окне браузера',
    'acct.ms_note': 'Требуется купленная лицензия Minecraft',

    // Misc
    'loading': 'Загрузка…',
    'error.retry': 'Попробовать снова',
    'launcher.subtitle': 'Minecraft Launcher by vlalikoffc',
    'dl.paused': '⏸ Пауза',
    'dl.resume': '▶ Продолжить',
    'dl.pause': '⏸ Пауза',
    'dl.cancel': '✕ Отмена',

    // Performance warnings
    'perf.warn.low': 'Мало потоков — загрузки будут медленными.',
    'perf.warn.high': 'Много потоков — возможны проблемы на медленных соединениях.',

    // Settings — game dir hint
    'settings.game_dir_hint': 'Общие библиотеки, ассеты и Java хранятся здесь. Сохранения инстансов в папке instances/.',

    // Settings — Customize tab
    'settings.tab.customize': 'Кастомизация',
    'customize.accent': 'Цвет акцента',
    'customize.accent_tip': 'Изменяет цвет подсветки во всём лаунчере — кнопка запуска, бейджи, слайдеры и всё остальное.',
    'customize.reset': 'Сбросить',
    'customize.apply_lb': 'Применить ко вкладке LiquidBounce',
    'customize.apply_lb_tip': 'Не рекомендуется — LiquidBounce использует синий как фирменный цвет. Другой акцент может не сочетаться с логотипом LiquidBounce.',
    'customize.bg_orbs': 'Цвет фоновых сфер',
    'customize.bg_orbs_tip': 'Изменяет цвет фоновых световых сфер.',

    // Settings — tooltip hints
    'settings.tip.ram': 'Минимум: 512 МБ. Рекомендуется: 2–4 ГБ для большинства версий. Слишком большое значение может ухудшить производительность, если в системе мало свободной RAM.',
    'settings.tip.concurrent': 'Количество параллельных потоков загрузки. Больше = быстрее, но может перегрузить медленное соединение или HDD.',
    'settings.tip.close_on_launch': 'Когда включено, лаунчер скрывается при запуске игры и появляется снова после её закрытия.',
    'settings.tip.reset_setup': 'Сбрасывает флаг завершения настройки — мастер запустится снова при следующем открытии. Аккаунты и инстансы не затрагиваются.',

    // Instance errors
    'error.prefix': 'Ошибка:',
    'inst.no_versions': 'Версий нет.',
    'inst.name_taken': 'Инстанс с таким именем уже существует.',
    'isettings.copy_log': 'Скопировать latest.log',
    'isettings.copied': 'Скопировано!',
    'reinstall.choose': 'Выберите, что сохранить при переустановке инстанса.',

    // Console toggle
    'settings.console': 'Окно консоли',
    'settings.console_hint': 'Показывать окно логов при запуске игры',

    // Mod loader picker
    'inst.loader.title': 'Загрузчик модов',
    'inst.loader.next': 'Далее',
    'inst.loader.vanilla': 'Ванилла',
    'inst.loader.fabric': 'Fabric',
    'inst.loader.vanilla_desc': 'Без модов, чистый Minecraft',
    'inst.loader.fabric_desc': 'Включает Fabric API с Modrinth',
    'inst.loader.soon': 'Скоро',

    // Console window
    'console.title': 'Консоль',
    'console.copy': 'Скопировать лог',
    'console.open_folder': 'Папка с логом',
    'console.empty': 'Ожидание вывода игры…',
    'console.lines': 'строк',
    'console.clear': 'Очистить',
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
