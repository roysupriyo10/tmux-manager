// src/utils/fuzzy.ts
export function fuzzyMatch(pattern: string, str: string): number {
  const patternLower = pattern.toLowerCase();
  const strLower = str.toLowerCase();

  if (patternLower === strLower) return Infinity; // Exact match gets highest priority
  if (strLower.startsWith(patternLower))
    return 1000 + (strLower.length - patternLower.length); // Prefix match gets high priority
  if (strLower.includes(patternLower))
    return 500 + (strLower.length - patternLower.length); // Substring match gets medium priority

  let score = 0;
  let patternIndex = 0;
  let consecutiveMatches = 0;

  for (let i = 0; i < strLower.length; i++) {
    if (
      patternIndex < patternLower.length &&
      strLower[i] === patternLower[patternIndex]
    ) {
      patternIndex++;
      consecutiveMatches++;
      score += consecutiveMatches * 5; // Consecutive matches are weighted more
    } else {
      consecutiveMatches = 0;
    }
  }

  if (patternIndex === patternLower.length) {
    return score; // All pattern characters were found
  }

  return 0; // Not all pattern characters were found
}

export function findBestMatch(
  pattern: string,
  candidates: string[],
): string | null {
  if (candidates.length === 0) return null;

  const matchScores = candidates.map((candidate) => ({
    candidate,
    score: fuzzyMatch(pattern, candidate),
  }));

  // Sort by score in descending order
  matchScores.sort((a, b) => b.score - a.score);

  // Return the candidate with the highest score if it's greater than 0
  return matchScores[0].score > 0 ? matchScores[0].candidate : null;
}
