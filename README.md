# bot-logo-v1

Discord bot + local worker tool for iteratively generating vector logo
concepts with AI. The bot relays context to/from Discord; all image
generation happens on whoever runs the worker locally.

Design doc: [`.claude/specs/2026-08-27-discord-logo-bot-design.md`](.claude/specs/2026-08-27-discord-logo-bot-design.md).

## Prerequisites

- Rust (`cargo`)
- A Discord bot application with a token (create one at
  https://discord.com/developers/applications, add a Bot, copy its Token),
  invited to your server with permission to send messages and attachments
  in the target channel
- For the built-in generation pipeline: a GPU is strongly recommended
  (works on CPU, just slowly)

## Running the bot (one person, hosts it continuously)

```bash
cp .env.example .env
# fill in DISCORD_BOT_TOKEN, DISCORD_PUBLIC_KEY, DISCORD_GUILD_ID,
# DISCORD_CHANNEL_ID, DISCORD_ALLOWED_ROLE_ID, LOGO_BRIEF
cargo run --release --bin bot
```

`DISCORD_ALLOWED_ROLE_ID` is the role that's allowed to get a worker
token — only members with that role can use the tool. `LOGO_BRIEF` is a
short one-time description of what you're going for (e.g. "logo for a
17-person software team, minimalist, geometric") — it's only used to seed
the very first generation, before there's any chat/reactions to go on.

This process needs to stay running (it serves the worker API and the
Discord slash commands) and its port needs to be reachable from the
internet — see the design doc's Deployment section for the Dokploy setup.

## Getting a token and running the worker (anyone on the team with the right role)

DM the bot `/token` — it replies privately with your personal token (only
works if you have the role configured in `DISCORD_ALLOWED_ROLE_ID`;
running it again issues a new token and invalidates your old one).

```bash
cp worker.env.example worker.env
# fill in BOT_API_URL and WORKER_TOKEN (the token /token gave you)
cargo run --release --bin worker candle-vtracer
```

This pulls the current chat and existing logo candidates from the bot,
generates one or more new ideas locally, shows you each result to accept
or retry, and posts the ones you accept to Discord.

Options:

- `--repeat <logo-name>` — regenerate an existing candidate's exact prompt
  with this pipeline/model, to compare against how it originally came out.

The built-in pipeline currently only generates with `stabilityai/sdxl-turbo`
(weights auto-download via `hf-hub` on first use) — a `--model` flag exists
but only accepts that same default right now. candle maps each Stable
Diffusion version to its own fixed set of component files rather than
accepting an arbitrary HuggingFace repo ID, so swapping checkpoints isn't
just a config change; it's a follow-up worth doing once someone actually
wants a different model. Use the `external` pipeline below in the meantime
for anything else.

## Using your own generation setup (Python, ComfyUI, an API, etc.)

If you'd rather use a different pipeline than the built-in one, skip it
entirely with the `external` pipeline:

```bash
cargo run --release --bin worker external -- python3 my_generate.py
```

Your command receives the prompt as an argument (and, for `--repeat` /
iterations, the prior candidate's SVG/PNG file paths) and must output the
path to a resulting SVG file. The review loop and posting to Discord work
exactly the same afterward, regardless of how the image was made — this is
the integration point for anyone with a ComfyUI workflow, a paid API, a
local LLM that writes SVG directly, or anything else.

## Notes

- Discord doesn't preview `.svg` files inline, so every posted message
  shows a rendered PNG with the original SVG attached alongside it.
- There's no "finalize" command — when the team converges on a favorite,
  that's just a normal conversation, not a bot feature.
