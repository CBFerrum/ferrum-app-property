// Property's page of the workspace's portal (Ferrum docs/plan/apps-ui.md §9.1) at /portal/home/: a
// lessee's leases with the rent invoiced (each invoice opens on ERP's "Your invoices"), the repairs
// they asked for, and a new request for a unit they rent now. Everything comes from
// /api/app/property/portal/…, which answers by the address the person signed in with.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { type FormEvent, useState } from "react";
import { Link, useNavigate } from "react-router";
import { api, errorMessage } from "@ferrum/client";
import { Badge, Button, EmptyState, ErrorState, Field, PageLoading, statusTone, toast, useShellTitle } from "@ferrum/kit";

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
  invoices: { period_from?: string | null; period_to?: string | null; amount?: string | null; invoice?: string | null }[];
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
  useShellTitle("Your home");
  const q = useHome();
  if (q.error) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  if (!q.data) return <PageLoading />;
  const { leases, requests } = q.data;
  return (
    <div className="stack np-page">
      <div className="page-head">
        <div>
          <h1 className="page-title">Your home</h1>
          <p className="page-sub">Your lease, the rent invoiced, and the repairs you asked for.</p>
        </div>
        {rentedNow(leases).length > 0 && (
          <Link className="btn btn-primary btn-md" to="/new">
            Ask for a repair
          </Link>
        )}
      </div>
      {!leases.length ? (
        <EmptyState title="No lease here yet" icon="building-2">
          <p className="muted">When a lease is made out to you, it is here with its rent.</p>
        </EmptyState>
      ) : (
        leases.map((l) => (
          <section key={l.id} className="card card-pad stack" aria-label={`Lease ${l.number ?? ""}`}>
            <header className="row" style={{ justifyContent: "space-between" }}>
              <h3>
                {l.building} · {l.unit_number}
              </h3>
              <Badge tone={statusTone(l.status)}>{l.status}</Badge>
            </header>
            <p className="faint">
              {l.number} · {l.starts_on} to {l.terminated_on ?? l.ends_on} · {l.rent} {String(l.frequency ?? "").toLowerCase()}
            </p>
            {l.invoices.length > 0 && (
              <ul className="stack">
                {l.invoices.map((i) => (
                  <li key={`${i.period_from}`}>
                    {i.invoice ? <a href={`/portal/invoices/${i.invoice}`}>{`${i.period_from} to ${i.period_to}`}</a> : `${i.period_from} to ${i.period_to}`} ·{" "}
                    {i.amount}
                  </li>
                ))}
              </ul>
            )}
          </section>
        ))
      )}
      {requests.length > 0 && (
        <section className="card card-pad stack" aria-label="Your repairs">
          <h3>Repairs you asked for</h3>
          <ul className="stack">
            {requests.map((r) => (
              <li key={r.id}>
                {r.number} · {r.summary} · <Badge tone={statusTone(r.status)}>{r.status}</Badge>
                {r.done_on ? ` · done ${r.done_on}` : ""}
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

export function PortalNewRequest() {
  useShellTitle("Ask for a repair");
  const navigate = useNavigate();
  const qc = useQueryClient();
  const q = useHome();
  const [unit, setUnit] = useState("");
  const [summary, setSummary] = useState("");
  const [details, setDetails] = useState("");
  const [priority, setPriority] = useState("Normal");
  const [busy, setBusy] = useState(false);
  if (q.error) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  if (!q.data) return <PageLoading />;
  const units = rentedNow(q.data.leases);
  const chosen = unit || units[0]?.unit || "";
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    try {
      await api.post(`${BASE}/requests`, { unit: chosen, summary: summary.trim(), details: details.trim(), priority });
      await qc.invalidateQueries({ queryKey: ["property", "portal", "home"] });
      toast("Your request is with the team", "success");
      navigate("/");
    } catch (err) {
      toast(errorMessage(err), "error");
    } finally {
      setBusy(false);
    }
  };
  return (
    <form className="card card-pad stack np-page" onSubmit={(e) => void submit(e)}>
      <h1 className="page-title">Ask for a repair</h1>
      <Field label="Where">
        <select className="input" value={chosen} onChange={(e) => setUnit(e.target.value)}>
          {units.map((l) => (
            <option key={l.id} value={l.unit ?? ""}>
              {l.building} · {l.unit_number}
            </option>
          ))}
        </select>
      </Field>
      <Field label="What needs fixing">
        <input className="input" value={summary} maxLength={200} required onChange={(e) => setSummary(e.target.value)} />
      </Field>
      <Field label="Details">
        <textarea className="input" value={details} rows={4} onChange={(e) => setDetails(e.target.value)} />
      </Field>
      <Field label="How soon">
        <select className="input" value={priority} onChange={(e) => setPriority(e.target.value)}>
          <option value="Low">When convenient</option>
          <option value="Normal">Soon</option>
          <option value="Urgent">Urgent: the home is not safe or usable</option>
        </select>
      </Field>
      <div className="row">
        <Button type="submit" variant="primary" loading={busy} disabled={!chosen || !summary.trim()}>
          Send
        </Button>
        <Link to="/">Back</Link>
      </div>
    </form>
  );
}
