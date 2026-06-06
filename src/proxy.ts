import { NextResponse, type NextRequest } from "next/server";

// Basic-auth gate for the admin UI + admin APIs (ADMIN_USER / ADMIN_PASS).
// Next 16 "proxy" convention (formerly the `middleware` file).
export const config = {
  matcher: ["/admin", "/admin/:path*", "/api/admin/:path*"],
};

export function proxy(req: NextRequest) {
  const user = process.env.ADMIN_USER;
  const pass = process.env.ADMIN_PASS;
  const header = req.headers.get("authorization");

  if (user && pass && header?.startsWith("Basic ")) {
    const decoded = atob(header.slice(6));
    const sep = decoded.indexOf(":");
    const u = decoded.slice(0, sep);
    const p = decoded.slice(sep + 1);
    if (u === user && p === pass) {
      return NextResponse.next();
    }
  }

  return new NextResponse("Authentication required", {
    status: 401,
    headers: { "WWW-Authenticate": 'Basic realm="korucha-fund admin"' },
  });
}
