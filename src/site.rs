//! Property's pages of the workspace's portal (Ferrum docs/plan/apps-ui.md §9.1), from
//! `/api/app/property/portal/…`. "Your home" at `/portal/home/`: a lessee's leases with the rent
//! invoiced, and the repairs they asked for. A lease's rent is the one in effect on the workspace's
//! day, its reviews counted. "Properties you own" at `/portal/owner/` (statements
//! included, once billed): the buildings
//! naming the caller's address as their owner's. Portal people hold no roles: their leases are those
//! of the customers their address belongs to, and a request they raise runs as the system after
//! these checks.

use ferrum_domain::util::{b, id, s, sys};
use ferrum_kernel::{Ctx, Doc, Error, Filter, Kernel, Result, Site, SiteRequest, SiteResponse, Tx, with_retry};
use serde_json::{Map, Value, json};
use uuid::Uuid;

pub struct PropertySite;

#[async_trait::async_trait]
impl Site for PropertySite {
    async fn handle(&self, k: &Kernel, req: SiteRequest) -> Result<SiteResponse> {
        let user = req.user.as_ref().ok_or_else(|| Error::Unauthorized("sign in first".into()))?;
        let segs: Vec<&str> = req.path.split('/').filter(|p| !p.is_empty()).collect();
        let (status, out) = match (req.method.as_str(), segs.as_slice()) {
            ("GET", ["portal", "home"]) => (200, home(k, user).await?),
            ("GET", ["portal", "owner"]) => (200, owner(k, user).await?),
            ("POST", ["portal", "requests"]) => (201, raise(k, user, &body(&req)?).await?),
            _ => return Err(Error::NotFound(format!("property endpoint {} {}", req.method, req.path))),
        };
        let mut r = SiteResponse::json(status, &out);
        r.headers.push(("cache-control".into(), "no-store".into()));
        Ok(r)
    }
}

fn body(req: &SiteRequest) -> Result<Map<String, Value>> {
    match serde_json::from_slice::<Value>(&req.body) {
        Ok(Value::Object(m)) => Ok(m),
        _ => Err(Error::invalid("the request body must be a JSON object")),
    }
}

/// A piece of text, trimmed, at most `max` characters; required unless `optional`.
fn text(body: &Map<String, Value>, key: &str, max: usize, optional: bool) -> Result<Option<String>> {
    let v = body.get(key).and_then(Value::as_str).map(str::trim).unwrap_or_default();
    if v.is_empty() {
        return if optional { Ok(None) } else { Err(Error::field(key, "is required")) };
    }
    if v.chars().count() > max {
        return Err(Error::field(key, format!("at most {max} characters")));
    }
    Ok(Some(v.to_string()))
}

/// The customers an address belongs to: the customer's own email, or one of its contacts'.
async fn lessees_of(k: &Kernel, sctx: &Ctx, tx: &mut Tx, email: &str) -> Result<Vec<Uuid>> {
    let mut out: Vec<Uuid> = k
        .find_tx(sctx, tx, "Customer", &[Filter::eq("email", email)], Some("created_at asc"), 20)
        .await?
        .iter()
        .filter(|c| !b(c, "disabled"))
        .filter_map(|c| id(c, "id"))
        .collect();
    if sctx.schema.entity("Contact").is_some_and(|c| c.field("emails").is_some() && c.field("customer").is_some()) {
        let contacts = k.find_tx(sctx, tx, "Contact", &[Filter::eq("emails.email", email)], Some("created_at asc"), 20).await?;
        out.extend(contacts.iter().filter_map(|c| id(c, "customer")));
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// The caller's leases past their draft, newest first.
async fn leases(k: &Kernel, sctx: &Ctx, tx: &mut Tx, email: &str) -> Result<Vec<Doc>> {
    let mut out = vec![];
    for c in lessees_of(k, sctx, tx, email).await? {
        out.extend(k.find_tx(sctx, tx, "Lease", &[Filter::eq("lessee", c.to_string())], Some("starts_on desc"), 50).await?);
    }
    out.retain(|l| s(l, "status") != Some("Draft"));
    Ok(out)
}

/// `GET portal/home`: the caller's leases, each with its unit, its rent in effect today and the rent
/// invoiced, and the repairs they asked for.
async fn home(k: &Kernel, user: &Ctx) -> Result<Value> {
    let email = user.email.to_lowercase();
    if email.is_empty() {
        return Ok(json!({"leases": [], "requests": []}));
    }
    let sctx = sys(user);
    let today = sctx.today();
    let mut tx = k.db.tenant_tx(user.tenant).await?;
    let mut out = vec![];
    for l in leases(k, &sctx, &mut tx, &email).await? {
        // Whole, with its reviews: a list's rows come without them.
        let lease = k.get_tx(&sctx, &mut tx, "Lease", id(&l, "id").ok_or_else(|| Error::internal("a lease without an id"))?, false).await?;
        let unit = match id(&l, "unit") {
            Some(u) => k.get_tx(&sctx, &mut tx, "Unit", u, false).await?,
            None => Doc::new(),
        };
        let building = match id(&unit, "building") {
            Some(b) => k.get_tx(&sctx, &mut tx, "Building", b, false).await?,
            None => Doc::new(),
        };
        let invoices = k.find_tx(&sctx, &mut tx, "RentInvoice", &[Filter::eq("lease", s(&l, "id").unwrap_or_default())], Some("period_from desc"), 120).await?;
        let mut lines = Vec::with_capacity(invoices.len());
        for i in &invoices {
            // It opens on ERP's "Your invoices" once issued: a draft is not the lessee's yet.
            let issued = match id(i, "sales_invoice") {
                Some(si) => {
                    k.find_tx(&sctx, &mut tx, "SalesInvoice", &[Filter::eq("id", si.to_string()), Filter::eq("docstatus", 1)], None, 1).await?.len() == 1
                }
                None => false,
            };
            lines.push(json!({"period_from": s(i, "period_from"), "period_to": s(i, "period_to"), "amount": s(i, "amount"),
                              "invoice": if issued { s(i, "sales_invoice") } else { None }, "issued": issued}));
        }
        out.push(json!({
            "id": s(&l, "id"), "number": s(&l, "number"), "status": s(&l, "status"),
            "unit": s(&l, "unit"), "unit_number": s(&unit, "unit_number"), "building": s(&building, "building_name"), "address": s(&building, "address"),
            "starts_on": s(&l, "starts_on"), "ends_on": s(&l, "ends_on"), "terminated_on": s(&l, "terminated_on"),
            "rent": crate::leases::rent_on(&lease, today).normalize().to_string(), "frequency": s(&l, "frequency"),
            "invoices": lines,
        }));
    }
    let requests = k.find_tx(&sctx, &mut tx, "RepairRequest", &[Filter::eq("reported_by", email.clone())], Some("created_at desc"), 50).await?;
    tx.commit().await?;
    let requests: Vec<Value> = requests
        .iter()
        .map(|r| json!({"id": s(r, "id"), "number": s(r, "number"), "unit": s(r, "unit"), "summary": s(r, "summary"), "priority": s(r, "priority"), "status": s(r, "status"), "done_on": s(r, "done_on")}))
        .collect();
    Ok(json!({"leases": out, "requests": requests}))
}

/// Months of rent an owner sees for each building, and repairs.
const MONTHS: usize = 12;
const REPAIRS: usize = 50;
/// Billed statements an owner sees for each building, the latest first.
const STATEMENTS: usize = 12;

/// `GET portal/owner`: the buildings naming the caller as their owner, each with its units and the
/// lease letting each now at its rent in effect today; the rent, charges and late fees invoiced,
/// less what was given back, and still owed, by the month their period starts in (issued invoices
/// only, the latest twelve months with any); and its repairs open or done in the last 90 days.
async fn owner(k: &Kernel, user: &Ctx) -> Result<Value> {
    let email = user.email.to_lowercase();
    if email.is_empty() {
        return Ok(json!({"properties": []}));
    }
    let tenant = user.tenant;
    let mut tx = k.db.tenant_tx(tenant).await?;
    let buildings: Vec<(Uuid, Option<String>, Option<String>)> = sqlx::query_as(
        "select id, building_name, address from e_building where tenant_id = $1 and lower(owner_email) = $2 order by building_name, id limit 100",
    )
    .bind(tenant)
    .bind(email)
    .fetch_all(&mut *tx)
    .await?;
    let ids: Vec<Uuid> = buildings.iter().map(|b| b.0).collect();
    let units: Vec<(Uuid, Value)> = sqlx::query_as(
        "select u.building, jsonb_build_object('id', u.id, 'unit_number', u.unit_number, 'unit_type', u.unit_type, 'area', trim_scale(u.area)::text, 'status', u.status, \
           'lease', case when l.id is null then null else jsonb_build_object('number', l.number, 'lessee', c.customer_name, 'starts_on', l.starts_on, \
             'ends_on', l.ends_on, 'leaving_on', l.leaving_on, 'frequency', l.frequency, \
             'rent', trim_scale(coalesce((select r.rent from e_lease__rent_reviews r where r.tenant_id = l.tenant_id and r.parent_id = l.id \
               and r.effective_from <= $3 order by r.effective_from desc limit 1), l.rent))::text) end) \
         from e_unit u left join e_lease l on l.tenant_id = u.tenant_id and l.unit = u.id and l.status = 'Active' \
         left join e_customer c on c.tenant_id = l.tenant_id and c.id = l.lessee \
         where u.tenant_id = $1 and u.building = any($2) order by u.unit_number, u.id",
    )
    .bind(tenant)
    .bind(&ids)
    .bind(user.today())
    .fetch_all(&mut *tx)
    .await?;
    let months: Vec<(Uuid, Option<String>, String, String)> = sqlx::query_as(
        "select u.building, to_char(r.period_from, 'YYYY-MM'), trim_scale(coalesce(sum(si.grand_total), 0))::text, trim_scale(coalesce(sum(si.outstanding_amount), 0))::text \
         from e_rent_invoice r join e_lease l on l.tenant_id = r.tenant_id and l.id = r.lease \
         join e_unit u on u.tenant_id = l.tenant_id and u.id = l.unit \
         join e_sales_invoice si on si.tenant_id = r.tenant_id and si.docstatus = 1 and si.id in (r.sales_invoice, r.late_fee_invoice, r.credit_note) \
         where r.tenant_id = $1 and u.building = any($2) group by 1, 2 order by 1, 2 desc",
    )
    .bind(tenant)
    .bind(&ids)
    .fetch_all(&mut *tx)
    .await?;
    let repairs: Vec<(Uuid, Value)> = sqlx::query_as(
        "select u.building, jsonb_build_object('number', r.number, 'unit_number', u.unit_number, 'summary', r.summary, 'priority', r.priority, \
           'status', r.status, 'done_on', r.done_on) \
         from e_repair_request r join e_unit u on u.tenant_id = r.tenant_id and u.id = r.unit \
         where r.tenant_id = $1 and u.building = any($2) and (r.status is distinct from 'Done' or r.done_on >= $3) \
         order by (r.status = 'Done'), r.created_at desc, r.id",
    )
    .bind(tenant)
    .bind(&ids)
    .bind(user.today() - chrono::Duration::days(90))
    .fetch_all(&mut *tx)
    .await?;
    // Statements once billed, and what is still to pay on each bill once accounts issued it.
    let statements: Vec<(Uuid, Value)> = sqlx::query_as(
        "select s.building, jsonb_build_object('number', s.number, 'period_from', s.period_from, 'period_to', s.period_to, \
           'collected', trim_scale(s.collected)::text, 'fee_percent', trim_scale(s.fee_percent)::text, 'management_fee', trim_scale(s.management_fee)::text, \
           'expenses', trim_scale(s.expenses_total)::text, 'payout', trim_scale(s.payout)::text, \
           'still_to_pay', case when pi.docstatus = 1 then trim_scale(pi.outstanding_amount)::text end) \
         from e_owner_statement s left join e_purchase_invoice pi on pi.tenant_id = s.tenant_id and pi.id = s.purchase_invoice \
         where s.tenant_id = $1 and s.building = any($2) and s.status = 'Billed' order by s.period_from desc, s.number desc",
    )
    .bind(tenant)
    .bind(&ids)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let properties: Vec<Value> = buildings
        .iter()
        .map(|(b, name, address)| {
            let rent: Vec<Value> = months
                .iter()
                .filter(|m| m.0 == *b)
                .take(MONTHS)
                .map(|(_, month, invoiced, owed)| json!({"month": month, "invoiced": invoiced, "owed": owed}))
                .collect();
            json!({"id": b, "name": name, "address": address, "units": of(&units, *b, usize::MAX), "rent": rent, "repairs": of(&repairs, *b, REPAIRS),
                   "statements": of(&statements, *b, STATEMENTS)})
        })
        .collect();
    Ok(json!({"properties": properties}))
}

/// Building `b`'s rows, at most `most` of them.
fn of(rows: &[(Uuid, Value)], b: Uuid, most: usize) -> Vec<Value> {
    rows.iter().filter(|r| r.0 == b).take(most).map(|r| r.1.clone()).collect()
}

/// `POST portal/requests` `{unit, summary, details?, priority?}`: a repair asked for in a unit the
/// caller rents now.
async fn raise(k: &Kernel, user: &Ctx, body: &Map<String, Value>) -> Result<Value> {
    let email = user.email.to_lowercase();
    if email.is_empty() {
        return Err(Error::Forbidden("this account has no email address to answer".into()));
    }
    let unit: Uuid = body.get("unit").and_then(Value::as_str).and_then(|u| u.parse().ok()).ok_or_else(|| Error::field("unit", "is required"))?;
    let summary = text(body, "summary", 200, false)?;
    let details = text(body, "details", 5_000, true)?;
    let priority = match body.get("priority").and_then(Value::as_str).map(str::trim).filter(|p| !p.is_empty()) {
        None => "Normal",
        Some(p @ ("Low" | "Normal" | "Urgent")) => p,
        Some(_) => return Err(Error::field("priority", "choose Low, Normal or Urgent")),
    };
    k.rate_limit(
        &format!("property|raise|{}|{}", user.tenant, user.user),
        std::time::Duration::from_secs(3600),
        20,
        "too many requests in an hour; try again later",
    )
    .await?;
    let sctx = sys(user);
    with_retry(|| async {
        let mut tx = k.db.tenant_tx(user.tenant).await?;
        let rents = leases(k, &sctx, &mut tx, &email).await?.iter().any(|l| s(l, "status") == Some("Active") && id(l, "unit") == Some(unit));
        if !rents {
            return Err(Error::field("unit", "is not a unit you rent now"));
        }
        let saved = k
            .insert_tx(
                &sctx,
                &mut tx,
                "RepairRequest",
                crate::obj(json!({"unit": unit.to_string(), "summary": summary, "details": details, "priority": priority, "reported_by": email})),
            )
            .await?;
        tx.commit().await?;
        Ok(json!({"id": s(&saved, "id"), "number": s(&saved, "number"), "status": s(&saved, "status"), "priority": s(&saved, "priority")}))
    })
    .await
}
