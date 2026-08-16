// Ambient types for the jest-dom matchers registered at runtime in
// vitest-setup.ts (`import '@testing-library/jest-dom/vitest'`). That setup
// file lives outside src/ and isn't part of the svelte-check TS program, so
// without this the `.toBeInTheDocument()`-style matchers type-error under
// `npm run check` even though they work fine at test-run time.
/// <reference types="@testing-library/jest-dom/vitest" />
