const PKCE_VERIFIER_KEY = "deploywerk_oidc_pkce_verifier";
const PKCE_NONCE_KEY = "deploywerk_oidc_nonce";
const PKCE_REDIRECT_KEY = "deploywerk_oidc_redirect_uri";

function base64UrlEncode(bytes: ArrayBuffer | Uint8Array): string {
  const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  let bin = "";
  for (let i = 0; i < u8.length; i++) bin += String.fromCharCode(u8[i]!);
  const b64 = btoa(bin);
  return b64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function randomVerifier(): string {
  const a = new Uint8Array(32);
  crypto.getRandomValues(a);
  return base64UrlEncode(a);
}

function randomNonce(): string {
  const a = new Uint8Array(16);
  crypto.getRandomValues(a);
  return base64UrlEncode(a);
}

export async function pkceChallengeFromVerifier(verifier: string): Promise<string> {
  const data = new TextEncoder().encode(verifier);
  const digest = await crypto.subtle.digest("SHA-256", data);
  return base64UrlEncode(digest);
}

export function buildRedirectUri(fallbackPath = "/login/oidc/callback"): string {
  return `${window.location.origin}${fallbackPath}`;
}

/** Start Authentik (OIDC) redirect. Stores verifier, nonce, and redirect_uri in sessionStorage. */
export async function beginAuthentikLogin(params: {
  authorizationEndpoint: string;
  clientId: string;
  scopes: string;
  apiRedirectUri?: string | null;
}): Promise<void> {
  const verifier = randomVerifier();
  const nonce = randomNonce();
  const challenge = await pkceChallengeFromVerifier(verifier);
  const redirectUri = (params.apiRedirectUri?.trim() || buildRedirectUri()).trim();

  sessionStorage.setItem(PKCE_VERIFIER_KEY, verifier);
  sessionStorage.setItem(PKCE_NONCE_KEY, nonce);
  sessionStorage.setItem(PKCE_REDIRECT_KEY, redirectUri);

  const u = new URL(params.authorizationEndpoint);
  u.searchParams.set("response_type", "code");
  u.searchParams.set("client_id", params.clientId);
  u.searchParams.set("redirect_uri", redirectUri);
  u.searchParams.set("scope", params.scopes || "openid profile email");
  u.searchParams.set("code_challenge", challenge);
  u.searchParams.set("code_challenge_method", "S256");
  u.searchParams.set("nonce", nonce);

  window.location.assign(u.toString());
}

export function consumeOidcSession(): {
  codeVerifier: string;
  nonce: string;
  redirectUri: string;
} | null {
  const codeVerifier = sessionStorage.getItem(PKCE_VERIFIER_KEY);
  const nonce = sessionStorage.getItem(PKCE_NONCE_KEY);
  const redirectUri = sessionStorage.getItem(PKCE_REDIRECT_KEY);
  if (!codeVerifier || !nonce || !redirectUri) return null;
  sessionStorage.removeItem(PKCE_VERIFIER_KEY);
  sessionStorage.removeItem(PKCE_NONCE_KEY);
  sessionStorage.removeItem(PKCE_REDIRECT_KEY);
  return { codeVerifier, nonce, redirectUri };
}
