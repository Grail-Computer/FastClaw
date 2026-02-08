# μEmployee Demo (anime.js)

This folder contains a small, standalone animation that visualizes how μEmployee behaves in Slack:

- user mentions the agent
- task gets queued
- tool calls run
- guardrails request approvals
- agent replies and uploads files

## Run Locally

From repo root:

```bash
cd demo
python3 -m http.server 4173
```

Then open `http://127.0.0.1:4173/`.

Pick a scenario variant:

- `http://127.0.0.1:4173/?variant=v01-thread-recap`
- `http://127.0.0.1:4173/?variant=v08-guardrail-approval`

## Record Video + GIF

Requirements:

- Node 18+
- `ffmpeg` on PATH (optional, but recommended)

```bash
cd demo
npm i
npx playwright install chromium
npm run record
```

Outputs are written to `demo/output/`:

- `microemployee-demo.mp4` (recommended for README linking)
- `microemployee-demo.png` (thumbnail)
- `microemployee-demo.gif` (preview)
- `microemployee-demo.webm` (raw Playwright copy)

Record all variants:

```bash
cd demo
npm run record:all
```

This writes `demo/output/variants/` and generates `demo/output/variants/index.html` for quick review.
