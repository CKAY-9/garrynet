# GarryNet Frontend

The GarryNet website and loading screen, built with [Next.js](https://nextjs.org) (App Router) and TypeScript. Deployment is covered in the [main README](../README.md); the design docs are in [docs/](../docs/README.md).

## Development

Needs Node.js 22 and a running backend (see the main README).

```sh
cp .env.example .env   # point API_URL and NEXT_PUBLIC_API_URL at the backend
npm install
npm run dev            # http://localhost:3000
```

- `npm run build && npm start`: production build and server (`output: "standalone"`, used by the Dockerfile)
- `npm run lint`: ESLint

`NEXT_PUBLIC_API_URL` is inlined into the browser bundle at build time, so rebuild after changing it.

## Layout

```
src/app/(site)/      pages with the side navigation (home, forum, store, support, users, admin, ...)
src/app/loading-screen/  the full-screen loading screen used by sv_loadingurl
src/app/api/         route handlers the browser polls (notifications)
src/components/      UI components, one folder each, with their SCSS modules and server actions
src/lib/             backend API clients, shared types, permissions and helpers
```

Pages fetch from the backend on the server through the client in `src/lib/backend.ts`. To act as the logged-in user, they read the httpOnly `garrynet_token` cookie and send it as a bearer token (`authHeaders()` in `src/lib/auth.ts`). Mutations go through server actions (`actions.ts` files) that call the backend the same way.

This Next.js version has breaking changes from older releases. Before relying on an API, check the docs bundled in `node_modules/next/dist/docs/` (see [AGENTS.md](./AGENTS.md)).
