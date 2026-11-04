//! Property's page of the workspace's portal (Ferrum docs/plan/apps-ui.md §9.1), "Your home" at
//! `/portal/home/`, from `/api/app/property/portal/…`: a lessee's leases with the rent invoiced, and
//! the repairs they asked for. Portal people hold no roles: their leases are those of the customers
//! their address belongs to, and a request they raise runs as the system after these checks.

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

/// `GET portal/home`: the caller's leases, each with its unit and the rent invoiced, and the repairs
/// they asked for.
async fn home(k: &Kernel, user: &Ctx) -> Result<Value> {
    let email = user.email.to_lowercase();
    if email.is_empty() {
        return Ok(json!({"leases": [], "requests": []}));
    }
    let sctx = sys(user);
    let mut tx = k.db.tenant_tx(user.tenant).await?;
    let mut out = vec![];
    for l in leases(k, &sctx, &mut tx, &email).await? {
        let unit = match id(&l, "unit") {
            Some(u) => k.get_tx(&sctx, &mut tx, "Unit", u, false).await?,
            None => Doc::new(),
        };
        let building = match id(&unit, "building") {
            Some(b) => k.get_tx(&sctx, &mut tx, "Building", b, false).await?,
            None => Doc::new(),
        };
        let invoices = k.find_tx(&sctx, &mut tx, "RentInvoice", &[Filter::eq("lease", s(&l, "id").unwrap_or_default())], Some("period_from desc"), 120).await?;
        out.push(json!({
            "id": s(&l, "id"), "number": s(&l, "number"), "status": s(&l, "status"),
            "unit": s(&l, "unit"), "unit_number": s(&unit, "unit_number"), "building": s(&building, "building_name"), "address": s(&building, "address"),
            "starts_on": s(&l, "starts_on"), "ends_on": s(&l, "ends_on"), "terminated_on": s(&l, "terminated_on"),
            "rent": s(&l, "rent"), "frequency": s(&l, "frequency"),
            "invoices": invoices.iter().map(|i| json!({"period_from": s(i, "period_from"), "period_to": s(i, "period_to"), "amount": s(i, "amount"), "invoice": s(i, "sales_invoice")})).collect::<Vec<_>>(),
        }));
    }
    let requests = k.find_tx(&sctx, &mut tx, "MaintenanceRequest", &[Filter::eq("reported_by", email.clone())], Some("created_at desc"), 50).await?;
    tx.commit().await?;
    let requests: Vec<Value> = requests
        .iter()
        .map(|r| json!({"id": s(r, "id"), "number": s(r, "number"), "unit": s(r, "unit"), "summary": s(r, "summary"), "priority": s(r, "priority"), "status": s(r, "status"), "done_on": s(r, "done_on")}))
        .collect();
    Ok(json!({"leases": out, "requests": requests}))
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
                "MaintenanceRequest",
                crate::obj(json!({"unit": unit.to_string(), "summary": summary, "details": details, "priority": priority, "reported_by": email})),
            )
            .await?;
        tx.commit().await?;
        Ok(json!({"id": s(&saved, "id"), "number": s(&saved, "number"), "status": s(&saved, "status"), "priority": s(&saved, "priority")}))
    })
    .await
}
