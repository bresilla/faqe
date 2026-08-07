# Deployment and preview serving

`faqe build` produces an ordinary static website. A production deployment
serves that generated directory through a static host, CDN, or established web
server. The `faqe serve` command is a development preview and live-rebuild tool;
it is not a production HTTP server.

Every generated route contains readable semantic fallback content before WASM
starts. The generated bootstrap exposes a polite loading status, dynamically
imports and awaits the embedded loader, removes the status after successful
mounting, and converts loader/WASM fetch, compilation, or instantiation failures
into a durable alert with a Retry button. A startup failure leaves the readable
fallback in place rather than replacing it with a blank application mount.
Retry uses process-local query parameters for the loader and WASM URLs so a
transient corrupt or missing response cannot remain pinned by immutable browser
caching.

When JavaScript is disabled, the bootstrap status remains hidden and the route's
navigation, single main heading, article content, and internal links remain
available. The package gate checks a JavaScript-disabled deep article in
Chromium and statically verifies the fallback landmark/heading contract in every
generated HTML shell.

## Public origin and deployment base

These settings solve different problems and must not be combined manually:

- `site_url` in `site.toml` is the absolute, public site origin, for example
  `https://example.com`. It supplies the scheme and host used by canonical,
  OpenGraph, sitemap, and feed URLs. Do not add the deployment subpath to it.
- `--base-url` is the root-relative path at which the generated files will be
  mounted, for example `/notes/`. It prefixes generated asset, route, feed, and
  metadata paths exactly once. It must start and end with `/` and cannot contain
  a query, fragment, traversal, origin, or unsafe path characters.

Examples:

```sh
# https://example.com/ with site_url = "https://example.com"
faqe build ./content --output ./dist

# https://example.com/notes/ with the same site_url
faqe build ./content --output ./dist --base-url /notes/
```

For the second build, a page such as `/about/` receives the canonical URL
`https://example.com/notes/about/`. The generated files still live below the
selected output directory; `--base-url` does not choose an output path.

## Preview server

Preview defaults to all interfaces on port 3000 and watches for content changes:

```sh
faqe serve ./content
```

To restrict preview to the local machine, bind explicitly to loopback:

```sh
faqe serve ./content --bind 127.0.0.1:3000
```

The default `0.0.0.0` binding exposes the preview on every reachable interface. The
preview server has no authentication, authorization, TLS termination, rate
limiting, proxy trust policy, virtual hosts, or access log. Use a firewall/VPN
boundary and never expose it directly to the public Internet.

The preview server intentionally supports only the subset needed for local
review: GET/HEAD, bounded request headers, single byte ranges for media, MIME
types used by generated content, immutable caching for fingerprinted assets,
security headers, and an eight-worker bounded queue. It does not currently
provide TLS, HTTP/2 or HTTP/3, ETags/conditional requests, gzip/Brotli transfer
compression, directory listings, uploads, or production observability.

Preview always generates at `/`; it does not model a non-root `--base-url`.
When a watched rebuild is invalid, the process reports the error and continues
serving the last valid generated tree. Use `--no-watch` when deterministic
manual rebuilding is preferable.

## Production host responsibilities

The production host should:

1. serve the generated directory at the same path passed to `--base-url`;
2. send the generated MIME types correctly, especially
   `application/wasm` for the WASM module;
3. preserve immutable caching for fingerprinted assets while allowing route
   shells and feeds to refresh;
4. provide TLS and the required security headers, including
   `frame-ancestors 'none'` in the HTTP Content-Security-Policy header; and
5. map unknown routes to the generated `404.html` without rewriting valid
   static route directories to the homepage.

Run `make package` before distributing a release binary. The package smoke test
builds from a temporary content copy, starts the generated site on a dynamic
loopback port, requires a real Chromium WASM startup, and verifies readable
fallback alerts for missing and corrupt WASM modules. It also proves retry
recovery, wrong-MIME WebAssembly fallback, and missing/corrupt site-bundle error
states.
