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
import { Button, ButtonLink, Field, HomeSkeleton, LoadFailed, PageHead, StatePanel, StatusBadge, toast } from "@ferrum/kit";
import { PRIORITY_TONES, UNIT_TONE } from "./Home";

// Lease.status and RepairRequest.status as stored (packages/property/entities).
const LEASE_TONES = { Draft: "neutral", Active: "green", Ended: "neutral", Terminated: "red" } as const;
const REPAIR_TONES = { Open: "amber", "In Progress": "blue", Done: "green" } as const;

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
  if (q.error) return <><PageHead title={title} description={sub} /><LoadFailed error={q.error} onRetry={() => void q.refetch()} /></>;
  if (!q.data) return <><PageHead title={title} description={sub} /><HomeSkeleton queues={2} /></>;
  const { leases, requests } = q.data;
  return (
    <>
      <PageHead
        title={title}
        description={sub}
        actions={rentedNow(leases).length > 0 ? <ButtonLink to="/new" variant="primary" icon="plus">{t("Ask for a repair")}</ButtonLink> : undefined}
      />
      {!leases.length ? (
        <StatePanel kind="empty" title={t("No lease here yet")}>{t("When a lease is made out to you, it is here with its rent.")}</StatePanel>
      ) : (
        <div className="grid gap-4">
          {leases.map((l) => (
            <section key={l.id} className="card card-pad stack min-w-0" aria-label={t("Lease {value}", { value: l.number ?? "" })}>
              <header className="grid gap-1">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <h2 className="text-base font-semibold">
                    {isolate(l.building ?? "")} · {isolate(l.unit_number ?? "")}
                  </h2>
                  <StatusBadge value={l.status} tones={LEASE_TONES} label={isolate(l.status ?? "")} />
                </div>
                <p className="text-xs text-muted-foreground">
                  {t("{number} · {starts_on} to", { number: l.number ?? "", starts_on: l.starts_on ?? "" })} {l.terminated_on ?? l.ends_on} · {isolate(l.rent ?? "")} {isolate(String(l.frequency ?? "").toLowerCase())}
                </p>
              </header>
              {l.invoices.length > 0 && (
                <div className="table-wrap" tabIndex={0} role="region" aria-label={t("Lease {value}", { value: l.number ?? "" })}>
                  <table className="table">
                    <thead>
                      <tr>
                        <th scope="col">{t("Period")}</th>
                        <th scope="col" className="num">{t("Amount")}</th>
                        <th scope="col">{t("Invoice")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {l.invoices.map((i) => {
                        const period = t("{period_from} to {period_to}", { period_from: String(i.period_from), period_to: String(i.period_to) });
                        return (
                          <tr key={`${i.period_from}`}>
                            <td className="nowrap">{period}</td>
                            <td className="num">{isolate(i.amount ?? "")}</td>
                            <td>{i.invoice ? <a href={`/portal/invoices/${i.invoice}`}>{t("Open")}</a> : null}{!i.issued && <span className="text-xs text-muted-foreground">{t("Not issued yet")}</span>}</td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              )}
            </section>
          ))}
        </div>
      )}
      {requests.length > 0 && (
        <section className="card card-pad stack mt-4 min-w-0" aria-label={t("Your repairs")}>
          <h2 className="text-base font-semibold">{t("Repairs you asked for")}</h2>
          <RepairTable rows={requests.map((r) => ({ number: r.number, unit_number: null, summary: r.summary, priority: r.priority, status: r.status, done_on: r.done_on }))} caption={t("Your repairs")} />
        </section>
      )}
    </>
  );
}

/** Repairs as a table: what needs fixing, its number, its status as a badge and when it was done. */
function RepairTable({ rows, caption }: { rows: OwnerProperty["repairs"]; caption: string }) {
  const unit = rows.some((r) => r.unit_number);
  return (
    <div className="table-wrap" tabIndex={0} role="region" aria-label={caption}>
      <table className="table">
        <thead>
          <tr>
            <th scope="col">{t("Repair")}</th>
            {unit && <th scope="col">{t("Unit")}</th>}
            <th scope="col">{t("What needs fixing")}</th>
            <th scope="col">{t("Status")}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr key={r.number ?? i}>
              <td className="nowrap">{isolate(r.number ?? "")}</td>
              {unit && <td>{isolate(r.unit_number ?? "")}</td>}
              <td>
                {isolate(r.summary ?? "")}
                {r.priority === "Urgent" && <> <StatusBadge value="Urgent" tones={PRIORITY_TONES} label={t("Urgent")} /></>}
              </td>
              <td>
                <StatusBadge value={r.status} tones={REPAIR_TONES} label={isolate(r.status ?? "")} />
                {r.done_on && <span className="text-xs text-muted-foreground">{t(" · done {done_on}", { done_on: r.done_on })}</span>}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
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
  if (q.error) return <>{head}<LoadFailed error={q.error} onRetry={() => void q.refetch()} /></>;
  if (!q.data) return <>{head}<HomeSkeleton queues={1} /></>;
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
    <>
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
    </>
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
  if (q.error) return <><PageHead title={title} description={sub} /><LoadFailed error={q.error} onRetry={() => void q.refetch()} /></>;
  if (!q.data) return <><PageHead title={title} description={sub} /><HomeSkeleton queues={2} /></>;
  const { properties } = q.data;
  return (
    <>
      <PageHead title={title} description={sub} />
      {!properties.length ? (
        <StatePanel kind="empty" title={t("No property here names you as its owner")}>{t("When the managers name you as a property's owner, it is here.")}</StatePanel>
      ) : (
        <div className="grid gap-4">
          {properties.map((p) => (
            <Owned key={p.id} p={p} />
          ))}
        </div>
      )}
    </>
  );
}

function Owned({ p }: { p: OwnerProperty }) {
  const let_ = p.units.filter((u) => u.status === "Let").length;
  return (
    <section className="card card-pad stack min-w-0" aria-label={p.name ?? t("Property")}>
      <header>
        <h2 className="text-base font-semibold">{isolate(p.name ?? "")}</h2>
        <p className="text-xs text-muted-foreground">{isolate([p.address, t("{let_} of {length} units let", { let_, length: p.units.length })].filter(Boolean).join(" · "))}</p>
      </header>
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
          <h3 className="text-base font-semibold">{t("Repairs")}</h3>
          <RepairTable rows={p.repairs} caption={t("Repairs")} />
        </>
      )}
    </section>
  );
}
