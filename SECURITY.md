# Security Policy

GarryNet handles logins, payments and game server credentials, so security reports are taken seriously.

## Reporting a Vulnerability

Please **do not open a public issue** for security problems.

Report them privately through GitHub: go to this repository's **Security** tab and choose **Report a vulnerability**. Include:

- what the issue is and what an attacker could do with it
- steps to reproduce, or a proof of concept
- the affected part (backend, frontend, addon) and the commit or version you tested

You'll get a response as soon as possible. Please give a reasonable amount of time for a fix before disclosing the issue publicly.

## Supported Versions

GarryNet is in active development and has no versioned releases yet. Fixes go to the `main` branch, so keep your deployment up to date with it.

## Scope

In scope: anything in this repository, including the backend API, the website, the loading screen and the Garry's Mod addon. Examples include authentication or permission bypasses, store payment or fulfillment abuse, server token or access key leaks, and stored XSS through forum posts, support tickets or uploads.

Out of scope: problems in a specific deployment's configuration (such as a weak `JWT_SECRET` or a missing HTTPS setup), and vulnerabilities in third-party services such as Steam or Stripe.
