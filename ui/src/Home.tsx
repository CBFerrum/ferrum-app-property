// The units by building, each let, vacant or under maintenance; the leases ending in the next 60
// days, at their end or on notice; and the requests still open, urgent first. Each opens its record.

import { useQuery } from "@tanstack/react-query";
import { api, docPath, todayIso, t, isolate } from "@ferrum/client";
import { ButtonLink, HomeSkeleton, LoadFailed, PageHead, Queue, QueueGrid, QueueItem, StatePanel, StatusBadge, Tile, TileGrid, type Tone } from "@ferrum/kit";

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

/** RepairRequest.priority as stored: an urgent request keeps its mark. */
export const PRIORITY_TONES: Readonly<Record<string, Tone>> = { Urgent: "red" };

/** The most a queue lists before "and N more" opens the list. */
const LIMIT = 5;

export function Home() {
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
  const title = t("Units");
  const head = (
    <PageHead title={title} actions={<ButtonLink to="/e/Lease/new" variant="primary" icon="plus">{t("New lease")}</ButtonLink>} />
  );
  if (q.isPending) return <>{head}<HomeSkeleton queues={2} /></>;
  if (q.isError) return <>{head}<LoadFailed error={q.error} onRetry={() => void q.refetch()} /></>;
  if (!q.data.buildings.length)
    return (
      <>
        {head}
        <StatePanel kind="empty" title={t("No properties yet")} action={<ButtonLink to="/e/Building/new" variant="primary" icon="plus">{t("Add a property")}</ButtonLink>}>
          {t("Add a property and its units, and they show here with the leases ending and the repairs waiting.")}
        </StatePanel>
      </>
    );
  const grouped = byBuilding(q.data.units);
  const ending = endingSoon(q.data.leases, todayIso());
  const open = [...q.data.requests].sort((a, b) => Number(b.priority === "Urgent") - Number(a.priority === "Urgent"));
  // What needs the person comes before the units, when there is any; when there is none, the two quiet lines follow the units.
  const queues = (
    <QueueGrid>
      <Queue title={t("Ending in 60 days")} count={ending.length} more={{ to: "/e/Lease", count: ending.length - LIMIT }}>
        {ending.slice(0, LIMIT).map((l) => (
          <QueueItem
            key={l.id}
            to={`/e/Lease/${l.id}`}
            title={isolate(String(l.number ?? ""))}
            meta={isolate(l.leaving_on ? t("Ends {date}, on notice from the {value}", { date: lastDay(l), value: String(l.notice_by ?? "").toLowerCase() }) : t("Ends {date}", { date: lastDay(l) }))}
          />
        ))}
      </Queue>
      <Queue title={t("Open requests")} count={open.length} tone={open.some((r) => r.priority === "Urgent") ? "attention" : "default"} more={{ to: "/e/RepairRequest", count: open.length - LIMIT }}>
        {open.slice(0, LIMIT).map((r) => (
          <QueueItem
            key={r.id}
            to={`/e/RepairRequest/${r.id}`}
            title={isolate(String(r.summary ?? ""))}
            meta={isolate(String(r.number ?? ""))}
            trailing={r.priority === "Urgent" ? <StatusBadge value="Urgent" tones={PRIORITY_TONES} label={t("Urgent")} /> : undefined}
          />
        ))}
      </Queue>
    </QueueGrid>
  );
  const needsYou = ending.length > 0 || open.length > 0;
  return (
    <>
      {head}
      {needsYou && queues}
      <div className={`grid gap-6 ${needsYou ? "mt-6" : ""}`}>
        {q.data.buildings.map((b) => {
          const units = grouped.get(b.id) ?? [];
          const id = `property-${b.id}`;
          return (
            <section key={b.id} aria-labelledby={id} className="grid gap-3">
              <div className="flex items-baseline gap-2">
                <h2 id={id} className="text-base font-semibold">{isolate(String(b.building_name))}</h2>
                <span className="text-sm tabular-nums text-muted-foreground">{units.length}</span>
              </div>
              {units.length ? (
                <TileGrid label={String(b.building_name)}>
                  {units.map((u) => (
                    <Tile
                      key={u.id}
                      to={`/e/Unit/${u.id}`}
                      title={isolate(String(u.unit_number))}
                      meta={u.unit_type ? <span>{isolate(String(u.unit_type))}</span> : undefined}
                      status={<StatusBadge value={u.status} tones={UNIT_TONE} />}
                    />
                  ))}
                </TileGrid>
              ) : (
                <p className="text-sm text-muted-foreground">{t("No units yet")}</p>
              )}
            </section>
          );
        })}
      </div>
      {!needsYou && <div className="mt-6">{queues}</div>}
    </>
  );
}
