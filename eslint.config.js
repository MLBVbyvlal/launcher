// Flat config (ESLint 9). Scope: the React/TS frontend only — Rust is
// covered by clippy in CI. Kept deliberately small: type-aware rules would
// need a second tsconfig pass and slow the CI job for little gain here.
import js from '@eslint/js'
import tseslint from 'typescript-eslint'
import reactHooks from 'eslint-plugin-react-hooks'
import globals from 'globals'

export default tseslint.config(
  { ignores: ['dist/**', 'node_modules/**', 'src-tauri/**', 'vite.config.ts'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['src/**/*.{ts,tsx}'],
    plugins: { 'react-hooks': reactHooks },
    languageOptions: { globals: globals.browser },
    rules: {
      ...reactHooks.configs.recommended.rules,
      // `_`-prefixed names are the documented "intentionally unused" marker.
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
      // Setters-in-effects patterns are audited by hand; the rule is noisy
      // for Tauri event wiring (listen → setState) and stays a warning.
      'react-hooks/exhaustive-deps': 'warn',
    },
  },
)
