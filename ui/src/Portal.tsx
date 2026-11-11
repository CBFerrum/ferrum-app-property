// Property's pages of the workspace's portal (Ferrum docs/plan/apps-ui.md §9.1). At /portal/home/: a
// lessee's leases with the rent invoiced (each invoice opens on ERP's "Your invoices"), the repairs
// they asked for, and a new request for a unit they rent now. At /portal/owner/: an owner's
// buildings, with their units and leases, the rent invoiced and still owed, repairs, and the statements
// billed to the company for their share. Everything
// comes from /api/app/property/portal/…, which answers by the address the person signed in with.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { type FormEvent, useState } from "react";
import { Link, useNavigate } from "react-router";
import { api, errorMessage, t, isolate } from "@ferrum/client";
import { Badge, Button, ErrorState, Field, ListSkeleton, StatePanel, statusTone, toast, useShellTitle } from "@ferrum/kit";
import { UNIT_TONE } from "./Home";

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
  useShellTitle(t("Your home"));
  const q = useHome();
  if (q.error) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  if (!q.data) return <ListSkeleton className="np-page" />;
  const { leases, requests } = q.data;
  return (
    <div className="stack np-page">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("Your home")}</h1>
          <p className="page-sub">{t("Your lease, the rent invoiced, and the repairs you asked for.")}</p>
        </div>
        {rentedNow(leases).length > 0 && (
          <Link className="btn btn-primary btn-md" to="/new">
            {t("Ask for a repair")}
          </Link>
        )}
      </div>
      {!leases.length ? (
        <StatePanel kind="empty" title={t("No lease here yet")}>{t("When a lease is made out to you, it is here with its rent.")}</StatePanel>
      ) : (
        leases.map((l) => (
          <section key={l.id} className="card card-pad stack" aria-label={t("Lease {value}", { value: l.number ?? "" })}>
            <header className="row" style={{ justifyContent: "space-between" }}>
              <h3>
                {isolate(l.building ?? "")} · {isolate(l.unit_number ?? "")}
              </h3>
              <Badge tone={statusTone(l.status, LEASE_TONES)}>{isolate(l.status ?? "")}</Badge>
            </header>
            <p className="faint">
              {t("{number} · {starts_on} to", { number: l.number ?? "", starts_on: l.starts_on ?? "" })} {l.terminated_on ?? l.ends_on} · {isolate(l.rent ?? "")} {isolate(String(l.frequency ?? "").toLowerCase())}
            </p>
            {l.invoices.length > 0 && (
              <ul className="stack">
                {l.invoices.map((i) => (
                  <li key={`${i.period_from}`}>
                    {i.invoice ? <a href={`/portal/invoices/${i.invoice}`}>{t("{period_from} to {period_to}", { period_from: String(i.period_from), period_to: String(i.period_to) })}</a> : t("{period_from} to {period_to}", { period_from: String(i.period_from), period_to: String(i.period_to) })} ·{isolate(" ")}
                    {isolate(i.amount ?? "")}
                    {!i.issued && <span className="muted"> {t("· not issued yet")}</span>}
                  </li>
                ))}
              </ul>
            )}
          </section>
        ))
      )}
      {requests.length > 0 && (
        <section className="card card-pad stack" aria-label={t("Your repairs")}>
          <h3>{t("Repairs you asked for")}</h3>
          <ul className="stack">
            {requests.map((r) => (
              <li key={r.id}>
                {isolate(r.number ?? "")} · {isolate(r.summary ?? "")} · <Badge tone={statusTone(r.status, REPAIR_TONES)}>{isolate(r.status ?? "")}</Badge>
                {r.done_on ? t(" · done {done_on}", { done_on: r.done_on }) : ""}
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

export function PortalNewRequest() {
  useShellTitle(t("Ask for a repair"));
  const navigate = useNavigate();
  const qc = useQueryClient();
  const q = useHome();
  const [unit, setUnit] = useState("");
  const [summary, setSummary] = useState("");
  const [details, setDetails] = useState("");
  const [priority, setPriority] = useState("Normal");
  const [busy, setBusy] = useState(false);
  if (q.error) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  if (!q.data) return <ListSkeleton className="np-page" />;
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
    <form className="card card-pad stack np-page" onSubmit={(e) => void submit(e)}>
      <h1 className="page-title">{t("Ask for a repair")}</h1>
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
        <Link to="/">{t("Back")}</Link>
      </div>
    </form>
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
  useShellTitle(t("Properties you own"));
  const q = useQuery({
    queryKey: ["property", "portal", "owner"],
    queryFn: ({ signal }) => api.get<{ properties: OwnerProperty[] }>(`${BASE}/owner`, {}, signal),
  });
  if (q.error) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  if (!q.data) return <ListSkeleton className="np-page" />;
  const { properties } = q.data;
  return (
    <div className="stack np-page">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("Properties you own")}</h1>
          <p className="page-sub">{t("Their units and leases, the rent invoiced and still owed, and repairs.")}</p>
        </div>
      </div>
      {!properties.length ? (
        <StatePanel kind="empty" title={t("No property here names you as its owner")}>{t("When the managers name you as a property's owner, it is here.")}</StatePanel>
      ) : (
        properties.map((p) => <Owned key={p.id} p={p} />)
      )}
    </div>
  );
}

function Owned({ p }: { p: OwnerProperty }) {
  const let_ = p.units.filter((u) => u.status === "Let").length;
  return (
    <section className="card card-pad stack" aria-label={p.name ?? t("Property")}>
      <header>
        <h2>{isolate(p.name ?? "")}</h2>
        <p className="faint">{isolate([p.address, t("{let_} of {length} units let", { let_, length: p.units.length })].filter(Boolean).join(" · "))}</p>
      </header>
      <div className="table-wrap">
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
                  <Badge tone={UNIT_TONE[u.status ?? ""] ?? "amber"}>{isolate(u.status ?? "")}</Badge>
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
          <h3>{t("Rent")}</h3>
          <p className="faint">{t("Issued invoices, rent, charges and late fees, by the month their period starts.")}</p>
          <div className="table-wrap">
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
          <h3>{t("Statements")}</h3>
          <p className="faint">{t("What was collected for the days, less the management fee and the expenses charged to you, and your share.")}</p>
          <div className="table-wrap">
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
          <h3>{t("Repairs")}</h3>
          <ul className="stack">
            {p.repairs.map((r, i) => (
              <li key={r.number ?? i}>
                {isolate(r.number ?? "")} · {isolate(r.unit_number ?? "")} · {isolate(r.summary ?? "")} · <Badge tone={statusTone(r.status, REPAIR_TONES)}>{isolate(r.status ?? "")}</Badge>
                {r.priority === "Urgent" ? t(" · urgent") : ""}
                {r.done_on ? t(" · done {done_on}", { done_on: r.done_on }) : ""}
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}
