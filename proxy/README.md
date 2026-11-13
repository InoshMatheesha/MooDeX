# MooDeX RAWG proxy

A Vercel edge function that adds the RAWG API key on the server. The MooDeX app never contains the key.

## Deploy

1. Create a new RAWG key at https://rawg.io/apidocs. Revoke the v1 key (`3e485ee2...`); it was published in v1's git history and must be treated as compromised.
2. From this folder, run `vercel deploy --prod`.
3. In the Vercel project settings, add the environment variable `RAWG_API_KEY`, then redeploy.
4. In MooDeX, go to Settings > Discover, set **Source** to *MooDeX proxy*, and enter `https://<your-project>.vercel.app/api/rawg` as the proxy URL.

## What it allows

- Only `GET /api/rawg/games` and `GET /api/rawg/games/<id>`.
- Only the query parameters MooDeX uses.
- `page_size` is capped at 40.
- About 60 requests per minute per IP on each edge instance. For a global limit, back it with Vercel KV or Upstash.
- ETags pass through, so the app's stale-while-revalidate cache gets cheap `304` responses.
