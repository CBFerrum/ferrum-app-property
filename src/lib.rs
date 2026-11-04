//! Property (FERRUM-INDUSTRY-008): buildings and units, leases with rent and charges invoiced each
//! period through ERP, inspections, and maintenance requests.
//!
//! * `leases`: a unit has one active lease at a time; activating one lets the unit, ending one frees
//!   it; upkeep invoices each lease's periods once each, rent and charges, up to today and its end.
//! * `requests`: an urgent maintenance request takes its unit out of service until it is done.
//!
//! This app's own design; no code was copied (docs/PROVENANCE.md). Hooks are registered per entity,
//! so a workspace without the `property` package never runs them.

mod leases;
mod requests;

use chrono::{NaiveDate, Utc};
use ferrum_domain::reports::ReportSource;
use ferrum_domain::util::s;
use ferrum_kernel::{Ctx, Doc, Error, HookRegistry, Kernel, Result, Tx};
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

include!(concat!(env!("OUT_DIR"), "/packages.rs"));
include!(concat!(env!("OUT_DIR"), "/ui.rs"));

/// This app's manifest (`app.yaml`).
pub const MANIFEST: &str = include_str!("../app.yaml");

/// This app's metadata packages, embedded at build time.
pub fn packages() -> std::result::Result<Vec<ferrum_meta::Package>, ferrum_meta::MetaError> {
    ferrum_meta::catalog::load_embedded(EMBEDDED)
}

/// Registers this app's document hooks.
pub fn register(hooks: &mut HookRegistry) {
    hooks.register("Lease", Arc::new(leases::LeaseHook)).register("MaintenanceRequest", Arc::new(requests::RequestHook));
}

/// This app's reports: none of its own yet; rent is ERP's receivables.
pub fn reports() -> Vec<Arc<dyn ReportSource>> {
    vec![]
}

/// Upkeep: each active lease's due periods invoiced, and leases past their end ended.
pub async fn maintenance(k: &Kernel) -> Result<()> {
    maintenance_on(k, Utc::now().date_naive()).await
}

/// [`maintenance`] as of `today`.
pub async fn maintenance_on(k: &Kernel, today: NaiveDate) -> Result<()> {
    let mut tx = k.db.system_tx().await?;
    let tenants: Vec<Uuid> = sqlx::query_scalar("select id from tenant where status = 'active' order by id").fetch_all(&mut *tx).await?;
    tx.commit().await?;
    let mut first_error: Option<Error> = None;
    for tenant in tenants {
        match k.registry.schema(tenant).await {
            Ok(s) if s.packages.iter().any(|p| p == "property") => {
                if let Err(e) = leases::upkeep(k, tenant, s, today).await {
                    first_error.get_or_insert(e);
                }
            }
            Ok(_) => {}
            Err(e) => {
                first_error.get_or_insert(e);
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}

/// A date field.
pub(crate) fn date(doc: &Doc, k: &str) -> Option<NaiveDate> {
    s(doc, k).and_then(|v| NaiveDate::parse_from_str(v.get(..10).unwrap_or(v), "%Y-%m-%d").ok())
}

/// A unit's status from what holds it: an open urgent request takes it out of service; otherwise an
/// active lease lets it.
pub(crate) async fn refresh_unit(k: &Kernel, ctx: &Ctx, tx: &mut Tx, unit: Uuid) -> Result<()> {
    let (urgent, let_now): (bool, bool) = sqlx::query_as(
        "select exists(select 1 from e_maintenance_request where tenant_id = $1 and unit = $2 and priority = 'Urgent' and status <> 'Done'), \
                exists(select 1 from e_lease where tenant_id = $1 and unit = $2 and status = 'Active')",
    )
    .bind(ctx.tenant)
    .bind(unit)
    .fetch_one(&mut **tx)
    .await?;
    let status = if urgent {
        "Under Maintenance"
    } else if let_now {
        "Let"
    } else {
        "Vacant"
    };
    k.set_values_tx(ctx, tx, "Unit", unit, obj(json!({"status": status}))).await
}

/// A document from JSON.
pub(crate) fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interface_is_a_release_the_platform_takes() {
        let m = ferrum_apps::AppManifest::parse(MANIFEST).expect("app.yaml");
        assert_eq!(m.routes(), vec![("property".to_string(), vec![])]);
        let files = UI.iter().map(|(p, b)| (p.to_string(), b.to_vec())).collect();
        let ui = ferrum_apps::ui::prepare("property", files).expect("ui/dist is an interface release");
        assert!(UI.iter().any(|(p, _)| *p == "index.html") && !UI.iter().any(|(p, _)| p.starts_with('.')), "{}", ui.digest);
    }
}
