import { dedent } from 'ts-dedent';

export const lines = (strings: TemplateStringsArray, ...values: unknown[]) =>
  dedent(
    strings,
    ...values.map((v, i) => {
      if (!v) return '\0';
      if (/=\s*$/.test(strings[i])) return JSON.stringify(v);
      return String(v);
    })
  )
    .split('\n')
    .filter(l => !l.includes('\0'))
    .join('\n');
