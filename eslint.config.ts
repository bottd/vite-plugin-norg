import js from '@eslint/js';
import ts from 'typescript-eslint';
import eslintConfigPrettier from 'eslint-config-prettier';
import globals from 'globals';

export default ts.config(
  js.configs.recommended,
  ...ts.configs.strict,
  ...ts.configs.stylistic,
  {
    languageOptions: {
      globals: {
        ...globals.node,
      },
    },
  },
  {
    ignores: [
      'dist/**',
      'wasm/**',
      'pkg/**',
      'src/parser/target/**',
      'src/parser/pkg/**',
      'target/**',
      'node_modules/**',
      '*.tgz',
      'package-lock.json',
    ],
  },
  eslintConfigPrettier
);
