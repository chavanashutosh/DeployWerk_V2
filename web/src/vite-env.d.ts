/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** When `"true"`, `/pricing` requires a logged-in session. */
  readonly VITE_PRICING_REQUIRES_AUTH?: string;
  /** Absolute API origin for production/preview (no trailing slash), e.g. `http://127.0.0.1:8080`. */
  readonly VITE_API_URL?: string;
}
