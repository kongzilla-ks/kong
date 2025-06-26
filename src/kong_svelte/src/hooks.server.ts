import type { Handle } from '@sveltejs/kit';

// Content Security Policy configuration
const CSP_DIRECTIVES = {
  'default-src': ["'self'"],
  'script-src': [
    "'self'",
    "'unsafe-inline'", // Required for SvelteKit
    "'unsafe-eval'", // May be needed for some libraries
    "https://www.googletagmanager.com",
    "https://www.google-analytics.com",
    "https://ssl.google-analytics.com",
  ],
  'style-src': [
    "'self'",
    "'unsafe-inline'", // Required for Svelte component styles
    "https://fonts.googleapis.com",
  ],
  'font-src': [
    "'self'",
    "https://fonts.gstatic.com",
  ],
  'img-src': [
    "'self'",
    "data:",
    "blob:",
    "https:",
    "http:", // For development
  ],
  'connect-src': [
    "'self'",
    "https://api.kongswap.io", // KongSwap API
    "https://mainnet.solana.validationcloud.io",
    "wss://mainnet.solana.validationcloud.io", // Solana WebSocket endpoint
    "https://wiser-omniscient-thunder.solana-mainnet.quiknode.pro", // QuikNode Solana RPC
    "https://*.icp0.io", // IC API endpoints
    "https://icp0.io",
    "https://icp-api.io",
    "https://boundary.ic0.app",
    "https://ic0.app",
    "https://*.ic0.app",
    "wss://*.icp0.io", // IC WebSocket endpoints
    "wss://icp0.io",
    "https://www.google-analytics.com",
    "https://analytics.google.com",
    "https://stats.g.doubleclick.net",
    "https://region1.google-analytics.com",
    "https://*.google-analytics.com", // All Google Analytics domains
    // Add localhost for development
    ...(process.env.NODE_ENV === 'development' ? [
      "http://localhost:*",
      "ws://localhost:*",
      "http://127.0.0.1:*",
      "ws://127.0.0.1:*",
    ] : []),
  ],
  'frame-src': [
    "'self'",
    "https://nfid.one", // NFID authentication
    "https://identity.ic0.app", // Internet Identity
  ],
  'object-src': ["'none'"],
  'base-uri': ["'self'"],
  'form-action': ["'self'"],
  'frame-ancestors': ["'none'"],
  'manifest-src': ["'self'"],
  'worker-src': ["'self'", "blob:"],
  'media-src': ["'self'"],
};

// Convert CSP directives object to string
function generateCSP(): string {
  return Object.entries(CSP_DIRECTIVES)
    .map(([directive, sources]) => `${directive} ${sources.join(' ')}`)
    .join('; ');
}

export const handle: Handle = async ({ event, resolve }) => {
  const response = await resolve(event);

  // Security headers
  response.headers.set('X-Frame-Options', 'DENY');
  response.headers.set('X-Content-Type-Options', 'nosniff');
  response.headers.set('Referrer-Policy', 'strict-origin-when-cross-origin');
  response.headers.set('Permissions-Policy', 'geolocation=(), microphone=(), camera=()');
  
  // Set CSP header
  // Use Report-Only mode initially to test without breaking functionality
  if (process.env.NODE_ENV === 'production') {
    response.headers.set('Content-Security-Policy', generateCSP());
  } else {
    // More permissive CSP for development
    response.headers.set('Content-Security-Policy-Report-Only', generateCSP());
  }

  // HSTS (HTTP Strict Transport Security) - only in production
  if (process.env.NODE_ENV === 'production') {
    response.headers.set(
      'Strict-Transport-Security',
      'max-age=31536000; includeSubDomains; preload'
    );
  }

  return response;
};