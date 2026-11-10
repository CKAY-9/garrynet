# Architecture

Part of [GarryNet Design](DESIGN.md).

## Frontend

Next.js with TypeScript.

## Backend

Actix Web (Rust) with PostgreSQL. Files go to local disk or an S3-compatible bucket through `object_store`; see [STORAGE.md](STORAGE.md).

## Addon

Standard GLua addon for Garry's Mod, in `addon/`. See [ADDON.md](ADDON.md).
