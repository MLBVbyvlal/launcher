// Русские строки. Каждый ключ отсюда обязан быть и в en.ts.
const ru: Record<string, string> = {
  // Setup — welcome
  'sw.welcome.title': 'Добро пожаловать в MLBV',
  'sw.welcome.sub': 'Minecraft Launcher by vlalikoffc',
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
  'feat.instances': 'Менеджер версий',
  'feat.instances_d': 'Создавай неограниченное количество версий, каждая со своими модами, мирами, ресурспаками и настройками. Запускай Minecraft 1.7.10 и 1.21 бок о бок без конфликтов.',
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
  'tab.console': 'Консоль',
  'play': 'Играть',
  'play.lb': 'Запустить LiquidBounce',
  'stop': 'Остановить',
  'no_account': 'Выберите аккаунт',
  'no_instance': 'Создайте версию',
  'add_account': '+ Добавить аккаунт',
  'new_instance': '+ Добавить',
  'sidebar.account': 'Аккаунт',
  'sidebar.instances': 'Версии',
  'settings.language': 'Язык',
  'sidebar.expand': 'Развернуть',
  'sidebar.collapse': 'Свернуть',

  // "Other tab running"
  'running.other': 'Запущен другой инстанс: {0}',
  'busy': 'Запуск уже идёт — подождите',
  'play.queue': 'В очередь',
  'launch.queue': 'Очередь',
  'launch.queue.remove': 'Убрать из очереди',
  'launch.stage.prepare': 'Подготовка',
  'launch.stage.fetch': 'Получение данных',
  'launch.stage.download': 'Загрузка',
  'launch.stage.install': 'Установка',
  'launch.stage.wait': 'Ожидание',
  'launch.stage.launch': 'Запуск',
  'launch.stage.error': 'Ошибка',

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
  'settings.min_ram': 'Мин. RAM (Xms)',

  // Settings — java
  'settings.java': 'Java',
  'settings.java_auto': '↓ автоскачка',
  'settings.java_found': '✓ найдена',
  'settings.java_hint': 'Отсутствующие версии скачаются автоматически при запуске.',
  'settings.java_path': 'Свой исполняемый файл Java',
  'settings.java_path_ph': 'Пусто = автовыбор…',
  'settings.java_path_hint': 'Полный путь к java / javaw. Проверяется при запуске — если файла нет, используется автовыбор. Действует только на игру, не на установщик Forge/NeoForge.',
  'settings.jvm_args': 'Дополнительные JVM-аргументы',
  'settings.jvm_args_ph': '-XX:+UseG1GC …',
  'settings.jvm_args_hint': 'Через пробел, добавляются после стандартных — ваши побеждают при конфликте.',

  // Settings — about
  'settings.about': 'О лаунчере',
  'settings.check_updates': 'Проверить обновления',
  'settings.checking': 'Проверяем…',
  'settings.up_to_date': 'Обновлений нет',
  'settings.update_check_failed': 'Не удалось проверить обновления:',
  'settings.update_available': 'доступно',
  'settings.cf_key': 'API-ключ CurseForge',
  'settings.cf_key_ph': 'Вставьте ключ…',
  'settings.cf_key_hint': 'Необязательно. Нужен только для встроенного браузера CurseForge — Modrinth работает без него. Бесплатный ключ: console.curseforge.com.',
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
  'inst.modal.title_mc': 'Добавить версию Minecraft',
  'inst.modal.title_lb': 'Добавить LiquidBounce',
  'inst.modal.name': 'Название',
  'inst.modal.name_ph': 'Моя версия',
  'inst.modal.version': 'Версия',
  'inst.modal.branch': 'Ветка',
  'inst.modal.cancel': 'Отмена',
  'inst.modal.create': 'Добавить',
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
  'isettings.min_ram': 'Переопределение мин. RAM',
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
  'update.choose_format': 'Выберите установщик:',
  'update.format_exe': '.exe установщик',
  'update.format_exe_desc': 'Рекомендуется — быстрое тихое обновление.',
  'update.format_msi': '.msi пакет',
  'update.format_msi_desc': 'Установка через системный установщик Windows с мастером настройки.',

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
  'dl.paused': 'На паузе',
  'dl.resume': 'Продолжить',
  'dl.pause': 'Пауза',
  'dl.cancel': 'Отмена',

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
  'inst.name_taken': 'Версия с таким именем уже существует.',
  'isettings.copy_log': 'Скопировать latest.log',
  'isettings.copied': 'Скопировано!',
  'reinstall.choose': 'Выберите, что сохранить при переустановке инстанса.',

  // Console toggle
  'settings.console': 'Вкладка консоли при запуске',
  'settings.console_hint': 'Переключаться на вкладку «Консоль» при старте игры',

  // Mod loader picker
  'inst.loader.title': 'Загрузчик модов',
  'inst.loader.next': 'Далее',
  'inst.loader.vanilla': 'Ванилла',
  'inst.loader.fabric': 'Fabric',
  'inst.loader.vanilla_desc': 'Без модов, чистый Minecraft',
  'inst.loader.fabric_desc': 'Включает Fabric API с Modrinth',
  'inst.loader.quilt_desc': 'Модульный загрузчик, совместим с Fabric',
  'inst.loader.forge_desc': 'Классические моды, через установщик',
  'inst.loader.neoforge_desc': 'Современный форк Forge, через установщик',

  // Console window
  'console.title': 'Консоль',
  'console.copy': 'Скопировать лог',
  'console.open_folder': 'Папка с логом',
  'console.empty': 'Ожидание вывода игры…',
  'console.lines': 'строк',
  'console.clear': 'Очистить',
  'console.copied': 'Скопировано',
  'console.no_running': 'Игра не запущена — запустите инстанс, и его вывод появится здесь.',
  'migrate.title': 'Обновление 0.0.5 — конвертация версий',
  'migrate.subtitle': 'Требуется один раз — лаунчер не запустится, пока она не закончится.',
  'migrate.what': 'Что изменилось',
  'migrate.p1': 'Старые версии мешали нативные библиотеки всех загрузчиков в одну общую папку, из-за чего ломались запуски. Теперь у каждого инстанса свои чистые нативки.',
  'migrate.p2': 'Плавающие версии «Latest» убраны: каждый инстанс привязывается к той версии, что у вас установлена.',
  'migrate.p3': 'Ничего важного не трогаем: сейвы, моды, конфиги, скриншоты и настройки остаются на месте.',
  'migrate.plan_pin': 'Привязка версий',
  'migrate.plan_clean': 'Удаление устаревших файлов',
  'migrate.plan_empty': 'Конвертировать нечего — данные уже в новом формате. Нажмите «Продолжить».',
  'migrate.kind.natives': 'Устаревшие общие нативки',
  'migrate.kind.inst_versions': 'Старые копии версий в инстансах',
  'migrate.kind.part': 'Оборванные загрузки',
  'migrate.kind.temp': 'Старые пакеты обновлений',
  'migrate.lb_head': 'новейший билд',
  'migrate.convert': 'Конвертировать',
  'migrate.continue': 'Продолжить',
  'migrate.enter': 'Открыть лаунчер',
  'migrate.step_pin': 'Привязка версий…',
  'migrate.step_clean': 'Чистка устаревших файлов…',
  'migrate.step_verify': 'Проверка…',
  'migrate.done': 'Конвертация завершена. Освобождено: {0}.',
  'migrate.kept': 'Сохранено: сейвы, моды, конфиги, библиотеки, ассеты, Java — перекачивать ничего не нужно.',
  'migrate.error_pin': 'Не удалось привязать {0}: {1}',
  'migrate.err_lb_offline': 'нет сети — подключитесь один раз, чтобы привязать сборку LiquidBounce',
  'migrate.err_verify': 'Проверка не прошла — {0}. Нажмите «Ещё раз», чтобы повторить.',
  'migrate.err_clean': 'При чистке возникли ошибки: {0}',

  // Loader version picker (step 3)
  'inst.loader.ver.title': 'Версия загрузчика',
  'inst.loader.ver.loading': 'Загрузка версий…',
  'inst.loader.ver.none': 'Нет версий для этой версии Minecraft.',

  // Instance settings nav
  'isettings.nav.overview': 'Общее',
  'isettings.nav.mods': 'Моды',
  'isettings.nav.logs': 'Логи',

  // Mods panel
  'isettings.mods.download': 'Скачать моды',
  'isettings.mods.add_file': 'Добавить файл',
  'isettings.mods.open_folder': 'Открыть папку',
  'isettings.mods.delete_selected': 'Удалить выбранные',
  'isettings.mods.empty': 'Модов нет',
  'isettings.mods.check_updates': 'Проверить обновления',
  'isettings.mods.checking': 'Проверка…',
  'isettings.mods.update_all': 'Обновить всё ({0})',
  'isettings.mods.updating': 'Обновление…',
  'isettings.mods.export': 'Список модов',
  'isettings.mods.enable': 'Включить',
  'isettings.mods.disable': 'Выключить',
  'isettings.mods.disabled': 'выключен',
  'isettings.mods.unknown': 'добавлен вручную',

  // Mod browser (Modrinth + CurseForge)
  'mods.title': 'Скачать моды',
  'mods.search_ph': 'Поиск модов…',
  'mods.all_versions': 'все версии',
  'mods.install': 'Установить',
  'mods.installed': 'Установлено',
  'mods.load_more': 'Показать ещё',
  'mods.no_results': 'Ничего не найдено',
  'mods.back': '← Назад',
  'mods.needs_key': 'Вставьте свой API-ключ CurseForge в Настройки → Общие, чтобы листать CurseForge. Бесплатно на console.curseforge.com.',
  'mods.key_invalid': 'CurseForge отклонил API-ключ — проверьте его в Настройки → Общие.',

  // Delete instance dialog
  'delete_inst.title': 'Удалить версию',
  'delete_inst.from_list': 'Убрать из списка',
  'delete_inst.from_list_desc': 'Убирает из списка лаунчера. Файлы игры остаются на диске.',
  'delete_inst.from_disk': 'Удалить с диска',
  'delete_inst.from_disk_desc': 'Безвозвратно удаляет все файлы игры. Нельзя отменить.',
  'delete_inst.confirm': 'Подтвердить',

  // Stop game warning
  'stop.warn.title': 'Остановить игру?',
  'stop.warn.body': 'Игра будет принудительно завершена.\nНесохранённый прогресс потеряется.',
  'stop.warn.yes': 'Да, стоп',
  'stop.warn.yes_cd': 'Да, стоп ({0}с)',
  'stop.warn.no': 'Нет, отмена',

  // Unstable loader version warning
  'loader.unstable.title': 'Нестабильная версия!',
  'loader.unstable.body': '— это бета/пре-релиз.\nМогут быть баги, крэши и несовместимость.\nТы уверен?',
  'loader.unstable.confirm': 'Да, хочу',
  'loader.unstable.confirm_cd': 'Да, хочу ({0}с)',
  'loader.unstable.cancel': 'ААА, ОТМЕНА!!',
  'loader.filter.releases': 'Релизы',
  'loader.filter.all': 'Все',
  'loader.no_releases': 'Нет релизов для этой версии MC',
  'loader.beta_label': 'бета',

  // Launch status

  // Update modal — unstable warning
  'update.unstable_warn': '⚠ Нестабильная версия.',
  'update.unstable_body': 'Это бета или пре-релиз — может содержать серьёзные баги и нестабильное поведение. Рекомендуем дождаться стабильного релиза.',

  // LB Configs panel
  'lb.back': '← Назад',
  'lb.loading': 'Загрузка…',
  'lb.empty': 'Конфигов нет',
  'lb.install': '↓ Установить',
  'lb.installing': 'Устанавливаю…',
  'lb.no_lb_instances': 'Нет инстансов с LiquidBounce',
  'lb.no_desc': 'Нет описания',
  'lb.installed_badge': '✓ Установлен',
  'lb.update_badge': '⬆ Обновление',
  'lb.no_releases_mc': 'Нет релизов для этой версии MC',
}
export default ru
