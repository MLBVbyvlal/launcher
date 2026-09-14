// English strings. Every key here must also exist in ru.ts.
const en: Record<string, string> = {
  // Setup — welcome
  'sw.welcome.title': 'Welcome to MLBV',
  'sw.welcome.sub': 'Minecraft Launcher by vlalikoffc',
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
  'tab.console': 'Console',
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
  'running.other': 'Another instance is running: {0}',
  'busy': 'Another game is launching — please wait',
  'play.queue': 'Add to queue',
  'launch.queue': 'Queue',
  'launch.queue.remove': 'Remove from queue',
  'launch.stage.prepare': 'Preparing',
  'launch.stage.fetch': 'Fetching',
  'launch.stage.download': 'Downloading',
  'launch.stage.install': 'Installing',
  'launch.stage.wait': 'Waiting',
  'launch.stage.launch': 'Starting',
  'launch.stage.error': 'Failed',

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
  'settings.min_ram': 'Min RAM (Xms)',

  // Settings — java
  'settings.java': 'Java',
  'settings.java_auto': '↓ auto-download',
  'settings.java_found': '✓ found',
  'settings.java_hint': 'Missing versions download automatically on first launch.',
  'settings.java_path': 'Custom Java executable',
  'settings.java_path_ph': 'Empty = auto-detect…',
  'settings.java_path_hint': 'Full path to java / javaw. Checked at launch — falls back to auto-detect when missing. Applies to the game only, not the Forge/NeoForge installer.',
  'settings.jvm_args': 'Extra JVM arguments',
  'settings.jvm_args_ph': '-XX:+UseG1GC …',
  'settings.jvm_args_hint': 'Space-separated, added after the launcher defaults — yours win on conflict.',

  // Settings — about
  'settings.about': 'About',
  'settings.check_updates': 'Check for updates',
  'settings.checking': 'Checking…',
  'settings.up_to_date': "You're up to date",
  'settings.update_check_failed': 'Update check failed:',
  'settings.update_available': 'available',
  'settings.cf_key': 'CurseForge API key',
  'settings.cf_key_ph': 'Paste key here…',
  'settings.cf_key_hint': 'Optional. Needed only for the built-in CurseForge browser — Modrinth works without it. Free key at console.curseforge.com.',
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
  'isettings.min_ram': 'Min RAM Override',
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
  'update.choose_format': 'Choose installer:',
  'update.format_exe': '.exe installer',
  'update.format_exe_desc': 'Recommended — fast silent update.',
  'update.format_msi': '.msi package',
  'update.format_msi_desc': 'Installs through the Windows Installer service with a setup wizard.',

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
  'dl.paused': 'Paused',
  'dl.resume': 'Resume',
  'dl.pause': 'Pause',
  'dl.cancel': 'Cancel',

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
  'settings.console': 'Console tab on launch',
  'settings.console_hint': 'Switch to the Console tab when a game starts',

  // Mod loader picker
  'inst.loader.title': 'Mod Loader',
  'inst.loader.next': 'Next',
  'inst.loader.vanilla': 'Vanilla',
  'inst.loader.fabric': 'Fabric',
  'inst.loader.vanilla_desc': 'No mods, pure Minecraft',
  'inst.loader.fabric_desc': 'Includes Fabric API from Modrinth',
  'inst.loader.quilt_desc': 'Modular loader, Fabric-compatible',
  'inst.loader.forge_desc': 'Classic mods, installer-based',
  'inst.loader.neoforge_desc': 'Modern Forge fork, installer-based',

  // Console window
  'console.title': 'Console',
  'console.copy': 'Copy Log',
  'console.open_folder': 'Open Log Folder',
  'console.empty': 'Waiting for game output…',
  'console.lines': 'lines',
  'console.clear': 'Clear',
  'console.copied': 'Copied',
  'console.no_running': 'No game running — launch an instance and its output will appear here.',
  'migrate.title': 'Update 0.0.5 — version conversion',
  'migrate.subtitle': 'Required once — the launcher will not start until it finishes.',
  'migrate.what': 'What changed',
  'migrate.p1': 'Old versions mixed native libraries of every mod loader in one shared folder, which broke launches. Each instance now gets its own clean natives.',
  'migrate.p2': 'Rolling “Latest” versions are removed: every instance is pinned to the exact version you have installed.',
  'migrate.p3': 'Nothing you care about is touched: saves, mods, configs, screenshots and settings stay exactly where they are.',
  'migrate.plan_pin': 'Pin versions',
  'migrate.plan_clean': 'Delete stale files',
  'migrate.plan_empty': 'Nothing to convert — your data is already in the new format. Press continue to finish.',
  'migrate.kind.natives': 'Stale shared natives',
  'migrate.kind.inst_versions': 'Old per-instance version copies',
  'migrate.kind.part': 'Interrupted downloads',
  'migrate.kind.temp': 'Old update packages',
  'migrate.lb_head': 'newest build',
  'migrate.convert': 'Convert',
  'migrate.continue': 'Continue',
  'migrate.enter': 'Open launcher',
  'migrate.step_pin': 'Pinning versions…',
  'migrate.step_clean': 'Cleaning stale files…',
  'migrate.step_verify': 'Verifying…',
  'migrate.done': 'Conversion complete. Freed {0}.',
  'migrate.kept': 'Kept: saves, mods, configs, libraries, assets, Java — nothing to re-download.',
  'migrate.error_pin': 'Could not pin {0}: {1}',
  'migrate.err_lb_offline': 'no network — connect once to pin the LiquidBounce build',
  'migrate.err_verify': 'Verification failed — {0}. Press retry to run the conversion again.',
  'migrate.err_clean': 'Cleanup reported errors: {0}',

  // Loader version picker (step 3)
  'inst.loader.ver.title': 'Loader Version',
  'inst.loader.ver.loading': 'Fetching versions…',
  'inst.loader.ver.none': 'No versions available for this Minecraft version.',

  // Instance settings nav
  'isettings.nav.overview': 'Overview',
  'isettings.nav.mods': 'Mods',
  'isettings.nav.logs': 'Logs',

  // Mods panel
  'isettings.mods.download': 'Download Mods',
  'isettings.mods.add_file': 'Add from file',
  'isettings.mods.open_folder': 'Open Folder',
  'isettings.mods.delete_selected': 'Delete selected',
  'isettings.mods.empty': 'No mods installed',
  'isettings.mods.check_updates': 'Check updates',
  'isettings.mods.checking': 'Checking…',
  'isettings.mods.update_all': 'Update all ({0})',
  'isettings.mods.updating': 'Updating…',
  'isettings.mods.export': 'Mod list',
  'isettings.mods.enable': 'Enable',
  'isettings.mods.disable': 'Disable',
  'isettings.mods.disabled': 'disabled',
  'isettings.mods.unknown': 'added manually',

  // Mod browser (Modrinth + CurseForge)
  'mods.title': 'Download mods',
  'mods.search_ph': 'Search mods…',
  'mods.all_versions': 'all versions',
  'mods.install': 'Install',
  'mods.installed': 'Installed',
  'mods.load_more': 'Load more',
  'mods.no_results': 'Nothing found',
  'mods.back': '← Back',
  'mods.needs_key': 'Paste your CurseForge API key in Settings → General to browse CurseForge. Free at console.curseforge.com.',
  'mods.key_invalid': 'CurseForge rejected the API key — check it in Settings → General.',

  // Delete instance dialog
  'delete_inst.title': 'Delete Instance',
  'delete_inst.from_list': 'Remove from list',
  'delete_inst.from_list_desc': 'Removes from the launcher list only. Game files stay on disk.',
  'delete_inst.from_disk': 'Delete from disk',
  'delete_inst.from_disk_desc': 'Permanently deletes all game files. This cannot be undone.',
  'delete_inst.confirm': 'Confirm',

  // Stop game warning
  'stop.warn.title': 'Stop the game?',
  'stop.warn.body': 'The game will be forcefully terminated.\nUnsaved progress will be lost.',
  'stop.warn.yes': 'Yes, stop',
  'stop.warn.yes_cd': 'Yes, stop ({0}s)',
  'stop.warn.no': 'No, cancel',

  // Unstable loader version warning
  'loader.unstable.title': 'Unstable version!',
  'loader.unstable.body': '— is a beta/pre-release.\nMay have bugs, crashes and incompatibility.\nAre you sure?',
  'loader.unstable.confirm': 'Yes, use it',
  'loader.unstable.confirm_cd': 'Yes, use it ({0}s)',
  'loader.unstable.cancel': 'AAA, CANCEL!!',
  'loader.filter.releases': 'Releases',
  'loader.filter.all': 'All',
  'loader.no_releases': 'No releases for this Minecraft version',
  'loader.beta_label': 'beta',

  // Launch status

  // Update modal — unstable warning
  'update.unstable_warn': '⚠ Unstable version.',
  'update.unstable_body': 'This is a beta or pre-release — may contain serious bugs and unstable behavior. We recommend waiting for a stable release.',

  // LB Configs panel
  'lb.back': '← Back',
  'lb.loading': 'Loading…',
  'lb.empty': 'No configs',
  'lb.install': '↓ Install',
  'lb.installing': 'Installing…',
  'lb.no_lb_instances': 'No instances with LiquidBounce',
  'lb.no_desc': 'No description',
  'lb.installed_badge': '✓ Installed',
  'lb.update_badge': '⬆ Update',
  'lb.no_releases_mc': 'No releases for this MC version',
}
export default en
