// [...path].ts - Vercel edge proxy: holds the RAWG key server-side so it never ships in the app (spec 9.1).
export const config = { runtime: "edge" };

// Only the endpoints MooDeX uses; everything else is 404.
const ALLOWED_PATH = /^games(\/\d{1,9})?$/;
const ALLOWED_PARAMS = new Set(["search", "search_precise", "page", "page_size", "dates", "ordering"]);
const WINDOW_MS = 60_000;
const MAX_PER_WINDOW = 60;

// Best-effort per-IP limit per edge instance; use Vercel KV/Upstash for a global limit.
const hits = new Map<string, { n: number; reset: number }>();

function limited(ip: string): number | null {
  const now = Date.now();
  const h = hits.get(ip);
  if (!h || now > h.reset) {
    hits.set(ip, { n: 1, reset: now + WINDOW_MS });
    return null;
  }
  h.n += 1;
  return h.n > MAX_PER_WINDOW ? Math.ceil((h.reset - now) / 1000) : null;
}

export default async function handler(req: Request): Promise<Response> {
  if (req.method !== "GET") return new Response("method not allowed", { status: 405 });
  const url = new URL(req.url);
  const path = url.pathname.replace(/^\/api\/rawg\/?/, "");
  if (!ALLOWED_PATH.test(path)) return new Response("not found", { status: 404 });

  const key = process.env.RAWG_API_KEY;
  if (!key) return new Response("proxy not configured", { status: 503 });

  const ip = req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() || "unknown";
  const wait = limited(ip);
  if (wait !== null) return new Response("rate limited", { status: 429, headers: { "Retry-After": String(wait) } });

  const upstream = new URL(`https://api.rawg.io/api/${path}`);
  for (const [k, v] of url.searchParams) {
    if (ALLOWED_PARAMS.has(k)) upstream.searchParams.set(k, v.slice(0, 200));
  }
  const size = Number(upstream.searchParams.get("page_size") ?? "20");
  upstream.searchParams.set("page_size", String(Math.min(Math.max(size, 1), 40)));
  upstream.searchParams.set("key", key);

  const headers: Record<string, string> = { "User-Agent": "MooDeX-proxy/1.0" };
  const inm = req.headers.get("if-none-match");
  if (inm) headers["If-None-Match"] = inm;

  const r = await fetch(upstream.toString(), { headers });
  const out = new Headers({ "Content-Type": r.headers.get("content-type") ?? "application/json" });
  const etag = r.headers.get("etag");
  if (etag) out.set("ETag", etag);
  const retry = r.headers.get("retry-after");
  if (retry) out.set("Retry-After", retry);
  out.set("Cache-Control", "public, s-maxage=1800, stale-while-revalidate=86400");
  return new Response(r.status === 304 ? null : r.body, { status: r.status, headers: out });
}
