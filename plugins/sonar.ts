import type { KlintPlugin } from "../core/types";
import { noSingleCharClass } from "../rules/no-single-char-class";
import { preferAt } from "../rules/prefer-at";
import { preferNullishCoalescingAssign } from "../rules/prefer-nullish-coalescing-assign";
import { preferStringRaw } from "../rules/prefer-string-raw";
import { preferStringRawRegexp } from "../rules/prefer-string-raw-regexp";
import { preferStringReplaceall } from "../rules/prefer-string-replaceall";

export const sonarPlugin: KlintPlugin = {
  name: "sonar",
  rules: {
    "sonar/prefer-string-replaceall": "error",
    "sonar/prefer-string-raw-regexp": "error",
    "sonar/prefer-string-raw": "error",
    "sonar/prefer-nullish-coalescing-assign": "error",
    "sonar/no-single-char-class": "error",
    "sonar/prefer-at": "error",
  },
  implementations: {
    "sonar/prefer-string-replaceall": preferStringReplaceall,
    "sonar/prefer-string-raw-regexp": preferStringRawRegexp,
    "sonar/prefer-string-raw": preferStringRaw,
    "sonar/prefer-nullish-coalescing-assign": preferNullishCoalescingAssign,
    "sonar/no-single-char-class": noSingleCharClass,
    "sonar/prefer-at": preferAt,
  },
};
