// The units by building, each let, vacant or under maintenance; the leases ending in the next 60
// days, at their end or on notice; and the requests still open, urgent first. Each opens its record.

import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { api, docPath, todayIso, t, isolate } from "@ferrum/client";
import { Badge, ErrorState, ListSkeleton, useShellTitle } from "@ferrum/kit";

interface Row {
  id: string;
  [k: string]: unknown;
}

const rows = (entity: string, filters: unknown[], extra: Record<string, string | number> = {}) =>
  api.get<{ rows: Row[] }>(docPath(entity), { filters: JSON.stringify(filters), limit: 500, ...extra }).then((r) => r.rows);

/** A unit's status as a badge's tone. */
export const UNIT_TONE: Record<string, "green" | "amber" | "red"> = { Let: "green", Vacant: "amber", "Under Maintenance": "red" };

/** Units by building id, in the order given. */
export function byBuilding(units: Row[]): Map<string, Row[]> {
  const out = new Map<string, Row[]>();
  for (const u of units) {
    const b = String(u.building ?? "");
    out.set(b, [...(out.get(b) ?? []), u]);
  }
  return out;
}

/** A lease's last day: the day its notice runs out, when notice was given, else its end. */
export function lastDay(l: Row): string {
  return String(l.leaving_on ?? l.ends_on ?? "");
}

/** Active leases ending from `today` to `days` days on, at their end or on notice, soonest first. */
export function endingSoon(leases: Row[], today: string, days = 60): Row[] {
  const last = new Date(Date.parse(today) + days * 86_400_000).toISOString().slice(0, 10);
  return leases
    .filter((l) => l.status === "Active" && lastDay(l) !== "" && lastDay(l) >= today && lastDay(l) <= last)
    .sort((a, b) => lastDay(a).localeCompare(lastDay(b)));
}

export function Home() {
  useShellTitle(t("Property"));
  const q = useQuery({
    queryKey: ["property", "home"],
    queryFn: async () => {
      const [buildings, units, leases, requests] = await Promise.all([
        rows("Building", [], { sort: "building_name asc" }),
        rows("Unit", [], { sort: "unit_number asc" }),
        rows("Lease", [["status", "eq", "Active"]]),
        rows("RepairRequest", [["status", "!=", "Done"]]),
      ]);
      return { buildings, units, leases, requests };
    },
  });
  if (q.isPending) return <ListSkeleton className="np-page" />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  const grouped = byBuilding(q.data.units);
  const ending = endingSoon(q.data.leases, todayIso());
  const open = [...q.data.requests].sort((a, b) => Number(b.priority === "Urgent") - Number(a.priority === "Urgent"));
  return (
    <div className="np-page">
      <header className="row" style={{ justifyContent: "space-between" }}>
        <h2>{t("Units")}</h2>
        <Link to="/e/Lease/new">{t("New lease")}</Link>
      </header>
      {q.data.buildings.map((b) => (
        <section key={b.id} className="card card-pad stack" aria-label={String(b.building_name)}>
          <h3>{isolate(String(b.building_name))}</h3>
          <div className="np-columns">
            {(grouped.get(b.id) ?? []).map((u) => (
              <Link key={u.id} className="card card-pad stack" to={`/e/Unit/${u.id}`}>
                <strong>{isolate(String(u.unit_number))}</strong>
                <span className="faint">{isolate(String(u.unit_type ?? ""))}</span>
                <Badge tone={UNIT_TONE[String(u.status)] ?? "amber"}>{isolate(String(u.status))}</Badge>
              </Link>
            ))}
          </div>
        </section>
      ))}
      <div className="np-columns">
        <section className="card card-pad stack" aria-label={t("Leases ending soon")}>
          <h3>{t("Ending in 60 days ({length})", { length: ending.length })}</h3>
          <ul className="stack">
            {ending.map((l) => (
              <li key={l.id}>
                <Link to={`/e/Lease/${l.id}`}>
                  {isolate(String(l.number ?? ""))} {t("· ends")} {isolate(lastDay(l))}
                  {l.leaving_on ? t(" on notice from the {value}", { value: String(l.notice_by ?? "").toLowerCase() }) : ""}
                </Link>
              </li>
            ))}
          </ul>
        </section>
        <section className="card card-pad stack" aria-label={t("Open requests")}>
          <h3>{t("Open requests ({length})", { length: open.length })}</h3>
          <ul className="stack">
            {open.map((r) => (
              <li key={r.id}>
                <Link to={`/e/RepairRequest/${r.id}`}>
                  {isolate(String(r.number ?? ""))} · {isolate(String(r.summary))} {r.priority === "Urgent" ? t("· urgent") : ""}
                </Link>
              </li>
            ))}
          </ul>
        </section>
      </div>
    </div>
  );
}
