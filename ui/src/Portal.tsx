// Property's pages of the workspace's portal (Ferrum docs/plan/apps-ui.md §9.1). At /portal/home/: a
// lessee's leases with the rent invoiced (each invoice opens on ERP's "Your invoices"), the repairs
// they asked for, and a new request for a unit they rent now. At /portal/owner/: an owner's
// buildings, with their units and leases, the rent invoiced and still owed, repairs, and the statements
// billed to the company for their share. Everything
// comes from /api/app/property/portal/…, which answers by the address the person signed in with.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { type FormEvent, useState } from "react";
import { useNavigate } from "react-router";
import { api, errorMessage, t, isolate } from "@ferrum/client";
import { Button, ButtonLink, Field, HomeSection, HomeSkeleton, LoadFailed, PageHead, PageStack, Queue, QueueGrid, QueueItem, StatePanel, StatusBadge, toast, type Tone } from "@ferrum/kit";
import { PRIORITY_TONES, UNIT_TONE } from "./Home";

// Lease.status and RepairRequest.status as stored (packages/property/entities).
const LEASE_TONES: Readonly<Record<string, Tone>> = { Draft: "neutral", Active: "green", Ended: "neutral", Terminated: "red" };
const REPAIR_TONES: Readonly<Record<string, Tone>> = { Open: "amber", "In Progress": "blue", Done: "green" };

const BASE = "/api/app/property/portal";

export interface PortalLease {
  id: string;
  number?: string | null;
  status?: string | null;
  unit?: string | null;
  unit_number?: string | null;
  building?: string | null;
  address?: string | null;
  starts_on?: string | null;
  ends_on?: string | null;
  terminated_on?: string | null;
  rent?: string | null;
  frequency?: string | null;
  invoices: { period_from?: string | null; period_to?: string | null; amount?: string | null; invoice?: string | null; issued?: boolean }[];
}

export interface PortalRequest {
  id: string;
  number?: string | null;
  unit?: string | null;
  summary?: string | null;
  priority?: string | null;
  status?: string | null;
  done_on?: string | null;
}

interface Home {
  leases: PortalLease[];
  requests: PortalRequest[];
}

/** The units a lessee may ask repairs for: those of their active leases. */
export function rentedNow(leases: PortalLease[]): PortalLease[] {
  return leases.filter((l) => l.status === "Active" && l.unit);
}

const useHome = () => useQuery({ queryKey: ["property", "portal", "home"], queryFn: ({ signal }) => api.get<Home>(`${BASE}/home`, {}, signal) });

export function PortalHome() {
  const q = useHome();
  const title = t("Your home");
  const sub = t("Your lease, the rent invoiced, and the repairs you asked for.");
  if (q.error) return <PageStack><PageHead title={title} description={sub} /><LoadFailed error={q.error} onRetry={() => void q.refetch()} /></PageStack>;
  if (!q.data) return <PageStack><PageHead title={title} description={sub} /><HomeSkeleton queues={2} /></PageStack>;
  const { leases, requests } = q.data;
  return (
    <PageStack gap="lg">
      <PageHead
        title={title}
        description={sub}
        actions={rentedNow(leases).length > 0 ? <ButtonLink to="/new" variant="primary" icon="plus">{t("Ask for a repair")}</ButtonLink> : undefined}
      />
      {!leases.length ? (
        <StatePanel kind="empty" title={t("No lease here yet")}>{t("When a lease is made out to you, it is here with its rent.")}</StatePanel>
      ) : (
        leases.map((l) => (
          <HomeSection key={l.id} title={`${isolate(l.building ?? "")} · ${isolate(l.unit_number ?? "")}`}>
            <div className="grid gap-3">
              <p className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                <StatusBadge value={l.status} tones={LEASE_TONES} label={isolate(l.status ?? "")} />
                <span>{isolate(`${t("{number} · {starts_on} to", { number: l.number ?? "", starts_on: l.starts_on ?? "" })} ${l.terminated_on ?? l.ends_on} · ${l.rent ?? ""} ${String(l.frequency ?? "").toLowerCase()}`)}</span>
              </p>
              {l.invoices.length > 0 && (
                <QueueGrid>
                  <Queue title={t("Rent invoiced")} count={l.invoices.length}>
                    {l.invoices.map((i) => (
                      <QueueItem
                        key={`${i.period_from}`}
                        title={isolate(t("{period_from} to {period_to}", { period_from: String(i.period_from), period_to: String(i.period_to) }))}
                        meta={i.issued ? undefined : t("Not issued yet")}
                        trailing={<span className="num">{isolate(i.amount ?? "")}</span>}
                        action={i.invoice ? <ButtonLink href={`/portal/invoices/${i.invoice}`}>{t("Open")}</ButtonLink> : undefined}
                      />
                    ))}
                  </Queue>
                </QueueGrid>
              )}
            </div>
          </HomeSection>
        ))
      )}
      {requests.length > 0 && (
        <QueueGrid>
          <RepairQueue title={t("Repairs you asked for")} rows={requests.map((r) => ({ number: r.number, unit_number: null, summary: r.summary, priority: r.priority, status: r.status, done_on: r.done_on }))} />
        </QueueGrid>
      )}
    </PageStack>
  );
}

/** Repairs as plain rows: what needs fixing, its number (and unit), its status as a badge and when it was done. */
function RepairQueue({ title, rows }: { title: string; rows: OwnerProperty["repairs"] }) {
  return (
    <Queue title={title} count={rows.length}>
      {rows.map((r, i) => (
        <QueueItem
          key={r.number ?? i}
          title={isolate(r.summary ?? "")}
          meta={isolate([r.number, r.unit_number, r.done_on ? t("Done {done_on}", { done_on: r.done_on }) : ""].filter(Boolean).join(" · "))}
          trailing={
            <span className="inline-flex flex-wrap items-center justify-end gap-1">
              {r.priority === "Urgent" && <StatusBadge value="Urgent" tones={PRIORITY_TONES} label={t("Urgent")} />}
              <StatusBadge value={r.status} tones={REPAIR_TONES} label={isolate(r.status ?? "")} />
            </span>
          }
        />
      ))}
    </Queue>
  );
}

export function PortalNewRequest() {
  const navigate = useNavigate();
  const qc = useQueryClient();
  const q = useHome();
  const [unit, setUnit] = useState("");
  const [summary, setSummary] = useState("");
  const [details, setDetails] = useState("");
  const [priority, setPriority] = useState("Normal");
  const [busy, setBusy] = useState(false);
  const head = <PageHead title={t("Ask for a repair")} back={{ to: "/", label: t("Your home") }} />;
  if (q.error) return <PageStack>{head}<LoadFailed error={q.error} onRetry={() => void q.refetch()} /></PageStack>;
  if (!q.data) return <PageStack>{head}<HomeSkeleton queues={1} /></PageStack>;
  const units = rentedNow(q.data.leases);
  const chosen = unit || units[0]?.unit || "";
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    try {
      await api.post(`${BASE}/requests`, { unit: chosen, summary: summary.trim(), details: details.trim(), priority });
      await qc.invalidateQueries({ queryKey: ["property", "portal", "home"] });
      toast(t("Your request is with the team"), "success");
      navigate("/");
    } catch (err) {
      toast(errorMessage(err), "error");
    } finally {
      setBusy(false);
    }
  };
  return (
    <PageStack>
    {head}
    <form className="card card-pad stack property-form" onSubmit={(e) => void submit(e)}>
      <Field label={t("Where")}>
        <select className="input" value={chosen} onChange={(e) => setUnit(e.target.value)}>
          {units.map((l) => (
            <option key={l.id} value={l.unit ?? ""}>
              {isolate(l.building ?? "")} · {isolate(l.unit_number ?? "")}
            </option>
          ))}
        </select>
      </Field>
      <Field label={t("What needs fixing")}>
        <input className="input" value={summary} maxLength={200} required onChange={(e) => setSummary(e.target.value)} />
      </Field>
      <Field label={t("Details")}>
        <textarea className="input" value={details} rows={4} onChange={(e) => setDetails(e.target.value)} />
      </Field>
      <Field label={t("How soon")}>
        <select className="input" value={priority} onChange={(e) => setPriority(e.target.value)}>
          <option value="Low">{t("When convenient")}</option>
          <option value="Normal">{t("Soon")}</option>
          <option value="Urgent">{t("Urgent: the home is not safe or usable")}</option>
        </select>
      </Field>
      <div className="row">
        <Button type="submit" variant="primary" loading={busy} disabled={!chosen || !summary.trim()}>
          {t("Send")}
        </Button>
        <ButtonLink to="/" variant="ghost">{t("Back")}</ButtonLink>
      </div>
    </form>
    </PageStack>
  );
}

export interface OwnerUnit {
  id: string;
  unit_number?: string | null;
  unit_type?: string | null;
  area?: string | null;
  status?: string | null;
  lease?: {
    number?: string | null;
    lessee?: string | null;
    starts_on?: string | null;
    ends_on?: string | null;
    leaving_on?: string | null;
    rent?: string | null;
    frequency?: string | null;
  } | null;
}

export interface OwnerProperty {
  id: string;
  name?: string | null;
  address?: string | null;
  units: OwnerUnit[];
  rent: { month: string; invoiced: string; owed: string }[];
  repairs: { number?: string | null; unit_number?: string | null; summary?: string | null; priority?: string | null; status?: string | null; done_on?: string | null }[];
  statements?: OwnerStatement[];
}

/** "2026-08" as its month and year, written where the browser is. */
export interface OwnerStatement {
  number: string;
  period_from: string;
  period_to: string;
  collected: string;
  fee_percent?: string | null;
  management_fee: string;
  expenses: string;
  payout: string;
  /** What is still to pay on the bill once accounts issued it; none before. */
  still_to_pay?: string | null;
}

/** Where a statement's payout stands: waiting for accounts, paid, or what is still to pay. */
export function payoutState(s: OwnerStatement): string {
  if (s.still_to_pay === null || s.still_to_pay === undefined) return t("Waiting for accounts");
  return Number(s.still_to_pay) <= 0 ? "Paid" : t("{still_to_pay} to pay", { still_to_pay: s.still_to_pay });
}

export function monthName(month: string, locale?: string): string {
  const [y, m] = month.split("-").map(Number);
  if (!y || !m || m > 12) return month;
  return new Date(Date.UTC(y, m - 1, 1)).toLocaleDateString(locale, { month: "long", year: "numeric", timeZone: "UTC" });
}

/** An amount as the browser writes numbers; anything else as it came. */
export function amount(v?: string | null, locale?: string): string {
  const n = v == null || v === "" ? Number.NaN : Number(v);
  return Number.isFinite(n) ? n.toLocaleString(locale, { maximumFractionDigits: 6 }) : (v ?? "");
}

/** A lease's last day: the day its notice runs out, when notice was given, else its end. */
const until = (l: NonNullable<OwnerUnit["lease"]>) => (l.leaving_on ? t("{leaving_on}, on notice", { leaving_on: l.leaving_on }) : (l.ends_on ?? ""));

export function PortalOwner() {
  const q = useQuery({
    queryKey: ["property", "portal", "owner"],
    queryFn: ({ signal }) => api.get<{ properties: OwnerProperty[] }>(`${BASE}/owner`, {}, signal),
  });
  const title = t("Properties you own");
  const sub = t("Their units and leases, the rent invoiced and still owed, and repairs.");
  if (q.error) return <PageStack><PageHead title={title} description={sub} /><LoadFailed error={q.error} onRetry={() => void q.refetch()} /></PageStack>;
  if (!q.data) return <PageStack><PageHead title={title} description={sub} /><HomeSkeleton queues={2} /></PageStack>;
  const { properties } = q.data;
  return (
    <PageStack gap="lg">
      <PageHead title={title} description={sub} />
      {!properties.length ? (
        <StatePanel kind="empty" title={t("No property here names you as its owner")}>{t("When the managers name you as a property's owner, it is here.")}</StatePanel>
      ) : (
        properties.map((p) => <Owned key={p.id} p={p} />)
      )}
    </PageStack>
  );
}

function Owned({ p }: { p: OwnerProperty }) {
  const let_ = p.units.filter((u) => u.status === "Let").length;
  return (
    <HomeSection title={isolate(p.name ?? "")}>
     <div className="grid gap-4">
      <p className="text-xs text-muted-foreground">{isolate([p.address, t("{let_} of {length} units let", { let_, length: p.units.length })].filter(Boolean).join(" · "))}</p>
      <div className="table-wrap" tabIndex={0} role="region" aria-label={p.name ?? t("Property")}>
        <table className="table">
          <caption className="sr-only">{t("The units of {name}", { name: p.name ?? "" })}</caption>
          <thead>
            <tr>
              <th scope="col">{t("Unit")}</th>
              <th scope="col">{t("Status")}</th>
              <th scope="col">{t("Let to")}</th>
              <th scope="col">{t("Until")}</th>
              <th scope="col" className="num">
                {t("Rent")}
              </th>
            </tr>
          </thead>
          <tbody>
            {p.units.map((u) => (
              <tr key={u.id}>
                <td>
                  {isolate(u.unit_number ?? "")}
                  {u.unit_type && <span className="faint"> · {isolate(u.unit_type)}</span>}
                </td>
                <td>
                  <StatusBadge value={u.status} tones={UNIT_TONE} label={isolate(u.status ?? "")} />
                </td>
                <td>{u.lease?.lessee ?? ""}</td>
                <td>{u.lease ? until(u.lease) : ""}</td>
                <td className="num">{u.lease ? `${amount(u.lease.rent)} ${String(u.lease.frequency ?? "").toLowerCase()}` : ""}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {p.rent.length > 0 && (
        <>
          <h3 className="text-base font-semibold">{t("Rent")}</h3>
          <p className="text-xs text-muted-foreground">{t("Issued invoices, rent, charges and late fees, by the month their period starts.")}</p>
          <div className="table-wrap" tabIndex={0} role="region" aria-label={p.name ?? t("Property")}>
            <table className="table">
              <caption className="sr-only">{t("The rent of {name} by month", { name: p.name ?? "" })}</caption>
              <thead>
                <tr>
                  <th scope="col">{t("Month")}</th>
                  <th scope="col" className="num">
                    {t("Invoiced")}
                  </th>
                  <th scope="col" className="num">
                    {t("Still owed")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {p.rent.map((m) => (
                  <tr key={m.month}>
                    <td>{isolate(monthName(m.month))}</td>
                    <td className="num">{isolate(amount(m.invoiced))}</td>
                    <td className="num">{isolate(amount(m.owed))}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
      {(p.statements ?? []).length > 0 && (
        <>
          <h3 className="text-base font-semibold">{t("Statements")}</h3>
          <p className="text-xs text-muted-foreground">{t("What was collected for the days, less the management fee and the expenses charged to you, and your share.")}</p>
          <div className="table-wrap" tabIndex={0} role="region" aria-label={p.name ?? t("Property")}>
            <table className="table">
              <caption className="sr-only">{t("The statements of {name}", { name: p.name ?? "" })}</caption>
              <thead>
                <tr>
                  <th scope="col">{t("Days")}</th>
                  <th scope="col" className="num">
                    {t("Collected")}
                  </th>
                  <th scope="col" className="num">
                    {t("Fee")}
                  </th>
                  <th scope="col" className="num">
                    {t("Expenses")}
                  </th>
                  <th scope="col" className="num">
                    {t("Your share")}
                  </th>
                  <th scope="col">{t("Payout")}</th>
                </tr>
              </thead>
              <tbody>
                {(p.statements ?? []).map((s) => (
                  <tr key={s.number}>
                    <td>
                      {t("{number} · {period_from} to {period_to}", { number: s.number, period_from: s.period_from, period_to: s.period_to })}
                    </td>
                    <td className="num">{isolate(amount(s.collected))}</td>
                    <td className="num">
                      {isolate(amount(s.management_fee))}
                      {s.fee_percent ? ` (${s.fee_percent}%)` : ""}
                    </td>
                    <td className="num">{isolate(amount(s.expenses))}</td>
                    <td className="num">{isolate(amount(s.payout))}</td>
                    <td>{isolate(payoutState(s))}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
      {p.repairs.length > 0 && (
        <>
          <QueueGrid>
            <RepairQueue title={t("Repairs")} rows={p.repairs} />
          </QueueGrid>
        </>
      )}
     </div>
    </HomeSection>
  );
}
