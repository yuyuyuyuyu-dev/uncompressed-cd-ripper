// The Playwright provider is what fills in the empty CDPSession interface that
// vitest/browser declares. Only vite.config.ts imports the provider, and that
// file belongs to tsconfig.node.json, so without this reference the test files
// never see the augmentation.
/// <reference types="@vitest/browser-playwright" />
