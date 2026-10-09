# Documentation site

Ink's documentation uses Blume. From the repository root, with Node.js 22.19 or newer and Bun installed:

```sh
bun install --frozen-lockfile
bun run docs:dev
```

Open the local URL printed by Blume. To check the production output:

```sh
bun run docs:build
bun run docs:validate
bun run docs:preview
```

`blume.config.ts` defines navigation, branding and redirects. Put images in `public/images`, served at `/images/...`. `theme.css` contains the example layouts.

## Cloudflare Workers

The site builds to static files in `docs/dist`. Workers Static Assets serves those files, search data, headers and HTTP redirects. No Worker script or Blume server adapter is needed for this configuration.

For a Workers Builds project connected to this repository, use:

- Root directory: repository root.
- Build command: `bun run docs:build && bun run docs:validate`.
- Deploy command: `bun run --cwd docs wrangler deploy`.
- Worker name: `ink-docs`, matching `wrangler.jsonc`.
- Build variables: `NODE_VERSION=24.21.0` and `BUN_VERSION=1.4.2`.

Install dependencies with `bun install --frozen-lockfile`. Keep the repository root as the build root so Bun uses the workspace lockfile.

To check Cloudflare's serving behaviour locally after building:

```sh
bun run --cwd docs wrangler dev --port 8787
```

The installer route must return an HTTP redirect, not an HTML redirect page. Check `/install.sh`, `/components`, `/inputs` and an unknown route with `curl -I http://localhost:8787/<path>`.

To build and deploy manually:

```sh
bun run --cwd docs deploy
```

Check the deployment on its `workers.dev` URL, then connect `ink.noscroll.ing` as a custom domain. The `/install.sh` route remains on the same domain.
