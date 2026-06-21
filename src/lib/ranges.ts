// Parse a friendly page-range string like "1-3, 5, 8-10" into groups of 0-based
// page indices — one group per comma-separated token. Used by the Split screen,
// where each group becomes one output file.

export interface ParsedRanges {
  groups: number[][]; // 0-based indices, one inner array per output file
  error: string | null;
}

export function parseRanges(input: string, pageCount: number): ParsedRanges {
  const trimmed = input.trim();
  if (!trimmed) return { groups: [], error: "Enter at least one page or range." };

  const groups: number[][] = [];
  for (const rawToken of trimmed.split(",")) {
    const token = rawToken.trim();
    if (!token) continue;

    const dash = token.match(/^(\d+)\s*-\s*(\d+)$/);
    const single = token.match(/^(\d+)$/);

    if (single) {
      const n = Number(single[1]);
      if (n < 1 || n > pageCount)
        return { groups: [], error: `Page ${n} is out of range (1–${pageCount}).` };
      groups.push([n - 1]);
    } else if (dash) {
      let start = Number(dash[1]);
      let end = Number(dash[2]);
      if (start > end) [start, end] = [end, start];
      if (start < 1 || end > pageCount)
        return {
          groups: [],
          error: `Range ${token} is out of bounds (1–${pageCount}).`,
        };
      const arr: number[] = [];
      for (let p = start; p <= end; p++) arr.push(p - 1);
      groups.push(arr);
    } else {
      return { groups: [], error: `"${token}" isn't a valid page or range.` };
    }
  }

  if (groups.length === 0) return { groups: [], error: "Enter at least one page or range." };
  return { groups, error: null };
}

/** Preset: one file per page. */
export function eachPageSeparately(pageCount: number): number[][] {
  return Array.from({ length: pageCount }, (_, i) => [i]);
}
