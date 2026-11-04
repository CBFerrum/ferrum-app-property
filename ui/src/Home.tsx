// The units by building, each let, vacant or under maintenance; the leases ending in the next 60
// days; and the requests still open, urgent first. Each opens its record.

import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { api, docPath } from "@ferrum/client";
import { Badge, ErrorState, PageLoading, useShellTitle } from "@ferrum/kit";

interface Row {
  id: string;
  [k: string]: unknown;
}

const rows = (entity: string, filters: unknown[], extra: Record<string, string | number> = {}) =>
  api.get<{ rows: Row[] }>(docPath(entity), { filters: JSON.stringify(filters), limit: 500, ...extra }).then((r) => r.rows);

const TONE: Record<string, "green" | "amber" | "red"> = { Let: "green", Vacant: "amber", "Under Maintenance": "red" };

/** Units by building id, in the order given. */
export function byBuilding(units: Row[]): Map<string, Row[]> {
  const out = new Map<string, Row[]>();
  for (const u of units) {
    const b = String(u.building ?? "");
    out.set(b, [...(out.get(b) ?? []), u]);
  }
  return out;
}

/** Active leases ending from `today` to `days` days on, soonest first. */
export function endingSoon(leases: Row[], today: string, days = 60): Row[] {
  const last = new Date(Date.parse(today) + days * 86_400_000).toISOString().slice(0, 10);
  return leases
    .filter((l) => l.status === "Active" && typeof l.ends_on === "string" && l.ends_on >= today && l.ends_on <= last)
    .sort((a, b) => String(a.ends_on).localeCompare(String(b.ends_on)));
}

export function Home() {
  useShellTitle("Property");
  const q = useQuery({
    queryKey: ["property", "home"],
    queryFn: async () => {
      const [buildings, units, leases, requests] = await Promise.all([
        rows("Building", [], { sort: "building_name asc" }),
        rows("Unit", [], { sort: "unit_number asc" }),
        rows("Lease", [["status", "eq", "Active"]]),
        rows("MaintenanceRequest", [["status", "neq", "Done"]]),
      ]);
      return { buildings, units, leases, requests };
    },
  });
  if (q.isPending) return <PageLoading />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  const grouped = byBuilding(q.data.units);
  const ending = endingSoon(q.data.leases, new Date().toISOString().slice(0, 10));
  const open = [...q.data.requests].sort((a, b) => Number(b.priority === "Urgent") - Number(a.priority === "Urgent"));
  return (
    <div className="np-page">
      <header className="row" style={{ justifyContent: "space-between" }}>
        <h2>Units</h2>
        <Link to="/e/Lease/new">New lease</Link>
      </header>
      {q.data.buildings.map((b) => (
        <section key={b.id} className="card card-pad stack" aria-label={String(b.building_name)}>
          <h3>{String(b.building_name)}</h3>
          <div className="np-columns">
            {(grouped.get(b.id) ?? []).map((u) => (
              <Link key={u.id} className="card card-pad stack" to={`/e/Unit/${u.id}`}>
                <strong>{String(u.unit_number)}</strong>
                <span className="faint">{String(u.unit_type ?? "")}</span>
                <Badge tone={TONE[String(u.status)] ?? "amber"}>{String(u.status)}</Badge>
              </Link>
            ))}
          </div>
        </section>
      ))}
      <div className="np-columns">
        <section className="card card-pad stack" aria-label="Leases ending soon">
          <h3>Ending in 60 days ({ending.length})</h3>
          <ul className="stack">
            {ending.map((l) => (
              <li key={l.id}>
                <Link to={`/e/Lease/${l.id}`}>
                  {String(l.number ?? "")} · ends {String(l.ends_on)}
                </Link>
              </li>
            ))}
          </ul>
        </section>
        <section className="card card-pad stack" aria-label="Open requests">
          <h3>Open requests ({open.length})</h3>
          <ul className="stack">
            {open.map((r) => (
              <li key={r.id}>
                <Link to={`/e/MaintenanceRequest/${r.id}`}>
                  {String(r.number ?? "")} · {String(r.summary)} {r.priority === "Urgent" ? "· urgent" : ""}
                </Link>
              </li>
            ))}
          </ul>
        </section>
      </div>
    </div>
  );
}
