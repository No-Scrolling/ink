import http from "node:http";
import { createHash, randomBytes } from "node:crypto";

const port = Number(process.env.PORT ?? 8788);
const devices = new Map();
const codes = new Map();
const refreshTokens = new Set();
const token = () => randomBytes(24).toString("base64url");
const escape = value => String(value).replace(/[&<>"']/g, char => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[char]);
function json(response, value, status = 200) {
  response.writeHead(status, { "Content-Type": "application/json", "Cache-Control": "no-store" });
  response.end(JSON.stringify(value));
}
function page(response, body) {
  response.writeHead(200, { "Content-Type": "text/html; charset=utf-8", "Cache-Control": "no-store" });
  response.end(`<!doctype html><meta name="viewport" content="width=device-width"><title>Ink OAuth fixture</title><main><h1>Ink OAuth fixture</h1>${body}</main>`);
}
function issue(response) {
  const refresh = token(); refreshTokens.add(refresh);
  json(response, { access_token: token(), refresh_token: refresh, token_type: "Bearer", expires_in: 45 });
}

const server = http.createServer(async (request, response) => {
  try {
    const url = new URL(request.url, `http://${request.headers.host ?? `localhost:${port}`}`);
    let body = "";
    if (url.pathname === "/upload") {
      let bytes = 0;
      for await (const chunk of request) bytes += chunk.length;
      json(response, { bytes }); return;
    }
    if (request.method === "POST") {
      for await (const chunk of request) {
        body += chunk;
        if (body.length > 64 * 1024) { json(response, { error: "request_too_large" }, 413); return; }
      }
    }
    const fields = new URLSearchParams(body);
    if (url.pathname === "/device" && request.method === "POST") {
      const deviceCode = token(); const userCode = randomBytes(3).toString("hex").toUpperCase();
      devices.set(deviceCode, { userCode, clientId: fields.get("client_id"), expiresAt: Date.now() + 300_000, approved: false, polledAt: 0 });
      json(response, { device_code: deviceCode, user_code: userCode, verification_uri: `${url.origin}/verify`, verification_uri_complete: `${url.origin}/verify?user_code=${userCode}`, expires_in: 300, interval: 5 });
    } else if (url.pathname === "/verify") {
      page(response, `<form method="post" action="/approve"><label>Device code <input name="user_code" value="${escape(url.searchParams.get("user_code") ?? "")}" required></label><button>Authorise device</button></form>`);
    } else if (url.pathname === "/approve" && request.method === "POST") {
      const entry = [...devices.values()].find(device => device.userCode === fields.get("user_code")?.toUpperCase());
      if (!entry || entry.expiresAt <= Date.now()) { json(response, { error: "expired_token" }, 400); return; }
      entry.approved = true;
      page(response, "<p>Device authorised. Return to Ink.</p>");
    } else if (url.pathname === "/authorize") {
      if (url.searchParams.get("redirect_uri") !== "ink-template://oauth/callback" || url.searchParams.get("code_challenge_method") !== "S256" || !url.searchParams.get("state")) {
        json(response, { error: "invalid_request" }, 400); return;
      }
      const hidden = [...url.searchParams].map(([name, value]) => `<input type="hidden" name="${escape(name)}" value="${escape(value)}">`).join("");
      page(response, `<p>Authorise the Ink Template public client?</p><form method="post" action="/authorise">${hidden}<button>Authorise Ink</button></form>`);
    } else if (url.pathname === "/authorise" && request.method === "POST") {
      if (fields.get("redirect_uri") !== "ink-template://oauth/callback" || fields.get("code_challenge_method") !== "S256") { json(response, { error: "invalid_request" }, 400); return; }
      const code = token();
      codes.set(code, { challenge: fields.get("code_challenge"), clientId: fields.get("client_id"), expiresAt: Date.now() + 60_000 });
      const redirect = new URL("ink-template://oauth/callback");
      redirect.searchParams.set("code", code); redirect.searchParams.set("state", fields.get("state") ?? "");
      response.writeHead(302, { Location: redirect.href, "Cache-Control": "no-store" }); response.end();
    } else if (url.pathname === "/token" && request.method === "POST") {
      if (fields.get("grant_type") === "urn:ietf:params:oauth:grant-type:device_code") {
        const device = devices.get(fields.get("device_code"));
        if (!device || device.expiresAt <= Date.now() || device.clientId !== fields.get("client_id")) { json(response, { error: "expired_token" }, 400); return; }
        if (Date.now() - device.polledAt < 4500) { device.polledAt = Date.now(); json(response, { error: "slow_down" }, 400); return; }
        device.polledAt = Date.now();
        if (!device.approved) { json(response, { error: "authorization_pending" }, 400); return; }
        devices.delete(fields.get("device_code")); issue(response);
      } else if (fields.get("grant_type") === "authorization_code") {
        const code = codes.get(fields.get("code")); codes.delete(fields.get("code"));
        const challenge = createHash("sha256").update(fields.get("code_verifier") ?? "").digest("base64url");
        if (!code || code.expiresAt <= Date.now() || code.challenge !== challenge || code.clientId !== fields.get("client_id") || fields.get("redirect_uri") !== "ink-template://oauth/callback") { json(response, { error: "invalid_grant" }, 400); return; }
        issue(response);
      } else if (fields.get("grant_type") === "refresh_token" && refreshTokens.delete(fields.get("refresh_token"))) {
        issue(response);
      } else json(response, { error: "invalid_grant" }, 400);
    } else if (url.pathname === "/download") {
      const size = 4 * 1024 * 1024; const etag = '"ink-fixture-4m-v1"';
      const range = request.headers.range?.match(/^bytes=(\d+)-$/);
      const partial = Boolean(range && (!request.headers["if-range"] || request.headers["if-range"] === etag));
      const start = partial ? Number(range[1]) : 0;
      if (start >= size) { response.writeHead(416, { "Content-Range": `bytes */${size}` }); response.end(); return; }
      const headers = { "Content-Type": "application/octet-stream", "Accept-Ranges": "bytes", ETag: etag, "Content-Length": size - start };
      if (partial) headers["Content-Range"] = `bytes ${start}-${size - 1}/${size}`;
      response.writeHead(partial ? 206 : 200, headers);
      if (request.method === "HEAD") { response.end(); return; }
      let offset = start;
      const timer = setInterval(() => {
        if (response.destroyed) { clearInterval(timer); return; }
        if (response.writableNeedDrain) return;
        const count = Math.min(64 * 1024, size - offset);
        const chunk = Buffer.allocUnsafe(count);
        for (let index = 0; index < count; index++) chunk[index] = (offset + index) % 251;
        response.write(chunk); offset += count;
        if (offset === size) { clearInterval(timer); response.end(); }
      }, 100);
      response.on("close", () => clearInterval(timer));
    } else {
      page(response, "<p>Local development server. Open the Accounts example in Ink to begin.</p><p><a href='/verify'>Authorise a device code</a></p>");
    }
  } catch {
    if (!response.headersSent) json(response, { error: "server_error" }, 500);
    else response.destroy();
  }
});
server.listen(port, "0.0.0.0", () => console.log(`Ink fixture listening on http://localhost:${port}`));
