export const lines = (...parts: (string | false | null | undefined)[]) =>
  parts.filter((p): p is string => !!p).join('\n');
