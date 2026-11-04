//! Leases: a unit's days are held by one lease at a time. Activating a lease lets its unit; ending
//! one early frees it now, and one past its end is ended by upkeep. Upkeep invoices each lease's
//! periods once each, rent and the other charges, through ERP to the lessee, from its start up to
//! today and its last day.

use crate::{date, obj, refresh_unit};
use async_trait::async_trait;
use chrono::{Duration, Months, NaiveDate, Utc};
use ferrum_domain::util::{d, elevated, id, money, rows, s};
use ferrum_kernel::{Ctx, Doc, DocHook, Error, HookCtx, Kernel, Result};
use ferrum_meta::effective::Schema;
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

pub struct LeaseHook;

/// How many months a period of `frequency` lasts.
pub(crate) fn months_of(frequency: &str) -> u32 {
    match frequency {
        "Quarterly" => 3,
        "Yearly" => 12,
        _ => 1,
    }
}

/// The period starting `from`: its last day, cut at the lease's last day.
pub(crate) fn period_end(from: NaiveDate, months: u32, last: NaiveDate) -> Option<NaiveDate> {
    let next = from.checked_add_months(Months::new(months))?;
    Some((next - Duration::days(1)).min(last))
}

#[async_trait]
impl DocHook for LeaseHook {
    fn name(&self) -> &'static str {
        "property.lease"
    }

    async fn validate(&self, _hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        if let (Some(f), Some(t)) = (date(doc, "starts_on"), date(doc, "ends_on"))
            && t <= f
        {
            return Err(Error::field("ends_on", "is after the lease starts"));
        }
        if d(doc, "rent") <= Decimal::ZERO {
            return Err(Error::field("rent", "is above zero"));
        }
        Ok(())
    }

    async fn validate_change(&self, hc: &mut HookCtx<'_>, old: Option<&Doc>, doc: &Doc) -> Result<()> {
        if hc.ctx.is_system {
            return Ok(());
        }
        if let Some(o) = old
            && s(o, "status") != Some("Draft")
            && ["unit", "lessee", "starts_on", "ends_on", "rent", "frequency", "charges"].iter().any(|k| o.get(*k) != doc.get(*k))
        {
            return Err(Error::invalid("a lease past its draft keeps its terms: end it and make another"));
        }
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, params: &Value) -> Result<Option<Value>> {
        let lease = id(doc, "id").ok_or_else(|| Error::internal("a lease without an id"))?;
        let unit = id(doc, "unit").ok_or_else(|| Error::field("unit", "is required"))?;
        let system = elevated(hc.ctx);
        match name {
            "activate" => {
                let from = date(doc, "starts_on").ok_or_else(|| Error::field("starts_on", "is required"))?;
                let to = date(doc, "ends_on").ok_or_else(|| Error::field("ends_on", "is required"))?;
                // Any lease past its draft holds its days, to the day it ended early if it did.
                let other: Option<String> = sqlx::query_scalar(
                    "select number from e_lease where tenant_id = $1 and unit = $2 and status <> 'Draft' and id <> $3 \
                     and starts_on <= $5 and coalesce(terminated_on, ends_on) >= $4 limit 1",
                )
                .bind(hc.ctx.tenant)
                .bind(unit)
                .bind(lease)
                .bind(from)
                .bind(to)
                .fetch_optional(&mut **hc.tx)
                .await?;
                if let Some(n) = other {
                    return Err(Error::invalid(format!("the unit is let on some of these days, under {n}")));
                }
                hc.kernel.set_values_tx(&system, hc.tx, "Lease", lease, obj(json!({"status": "Active", "next_invoice_on": from.to_string()}))).await?;
                refresh_unit(hc.kernel, &system, hc.tx, unit).await?;
                Ok(Some(json!({"status": "Active"})))
            }
            "terminate" => {
                let on = params
                    .get("date")
                    .and_then(Value::as_str)
                    .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok())
                    .ok_or_else(|| Error::field("date", "is the lease's last day, YYYY-MM-DD"))?;
                if date(doc, "starts_on").is_some_and(|f| on < f) || on > Utc::now().date_naive() {
                    return Err(Error::field("date", "is from the lease's start to today"));
                }
                hc.kernel.set_values_tx(&system, hc.tx, "Lease", lease, obj(json!({"status": "Terminated", "terminated_on": on.to_string()}))).await?;
                refresh_unit(hc.kernel, &system, hc.tx, unit).await?;
                Ok(Some(json!({"status": "Terminated"})))
            }
            _ => Ok(None),
        }
    }
}

/// One workspace's upkeep as of `today`: each lease's due periods invoiced once each, up to its last
/// day, and active leases past their end ended.
pub(crate) async fn upkeep(k: &Kernel, tenant: Uuid, schema: Arc<Schema>, today: NaiveDate) -> Result<()> {
    let ctx = Ctx::system(tenant, schema);
    let mut tx = k.db.tenant_tx(tenant).await?;
    let settings: Option<(Option<Uuid>, Option<Uuid>)> =
        sqlx::query_as("select company, rent_item from e_property_settings where tenant_id = $1 limit 1").bind(tenant).fetch_optional(&mut *tx).await?;
    let Some((Some(company), Some(rent_item))) = settings else {
        tx.commit().await?;
        return Ok(());
    };
    let currency: Option<Uuid> = sqlx::query_scalar("select default_currency from e_company where tenant_id = $1 and id = $2")
        .bind(tenant)
        .bind(company)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
    let leases: Vec<Uuid> = sqlx::query_scalar(
        "select id from e_lease where tenant_id = $1 and status in ('Active', 'Terminated') \
         and ((next_invoice_on <= $2 and next_invoice_on <= coalesce(terminated_on, ends_on)) or (status = 'Active' and ends_on < $2)) order by id",
    )
    .bind(tenant)
    .bind(today)
    .fetch_all(&mut *tx)
    .await?;
    for lease in leases {
        let doc = k.get_tx(&ctx, &mut tx, "Lease", lease, true).await?;
        let (Some(mut from), Some(end)) = (date(&doc, "next_invoice_on"), date(&doc, "ends_on")) else { continue };
        let last = date(&doc, "terminated_on").map_or(end, |t| t.min(end));
        let months = months_of(s(&doc, "frequency").unwrap_or("Monthly"));
        let charges = rows(&doc, "charges");
        while from <= today && from <= last {
            let Some(to) = period_end(from, months, last) else { break };
            // A period is invoiced once: its row says so.
            let done: bool = sqlx::query_scalar("select exists(select 1 from e_rent_invoice where tenant_id = $1 and lease = $2 and period_from = $3)")
                .bind(tenant)
                .bind(lease)
                .bind(from)
                .fetch_one(&mut *tx)
                .await?;
            if !done {
                let line = |item: Option<&str>, rate: Decimal, what: &str| json!({"item": item, "qty": "1", "price_list_rate": money(rate), "discount_percent": "0", "description": format!("{what}, {from} to {to}")});
                let rent_item = rent_item.to_string();
                let mut items = vec![line(Some(&rent_item), d(&doc, "rent"), "Rent")];
                items.extend(charges.iter().map(|c| line(s(c, "item"), d(c, "amount"), s(c, "charge").unwrap_or("Charge"))));
                let amount = d(&doc, "rent") + charges.iter().map(|c| d(c, "amount")).sum::<Decimal>();
                let remarks = format!("Lease {}, {from} to {to}", s(&doc, "number").unwrap_or_default());
                let inv = k
                    .insert_tx(
                        &ctx,
                        &mut tx,
                        "SalesInvoice",
                        obj(json!({"company": company.to_string(), "customer": s(&doc, "lessee"), "currency": currency.map(|c| c.to_string()), "items": items, "remarks": remarks})),
                    )
                    .await?;
                k.insert_tx(
                    &ctx,
                    &mut tx,
                    "RentInvoice",
                    obj(json!({"lease": lease.to_string(), "period_from": from.to_string(), "period_to": to.to_string(), "amount": money(amount), "sales_invoice": inv.get("id")})),
                )
                .await?;
            }
            from = to + Duration::days(1);
        }
        let mut values = obj(json!({"next_invoice_on": from.to_string()}));
        let ended = s(&doc, "status") == Some("Active") && end < today;
        if ended {
            values.insert("status".into(), json!("Ended"));
        }
        k.set_values_tx(&ctx, &mut tx, "Lease", lease, values).await?;
        if ended && let Some(unit) = id(&doc, "unit") {
            refresh_unit(k, &ctx, &mut tx, unit).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{months_of, period_end};
    use chrono::NaiveDate;

    #[test]
    fn periods_end_the_day_before_the_next_or_with_the_lease() {
        let day = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        assert_eq!(period_end(day("2026-01-01"), months_of("Monthly"), day("2026-12-31")), Some(day("2026-01-31")));
        assert_eq!(period_end(day("2026-01-31"), months_of("Monthly"), day("2026-12-31")), Some(day("2026-02-27")), "a month on from the 31st is the 28th");
        assert_eq!(period_end(day("2026-01-01"), months_of("Quarterly"), day("2026-12-31")), Some(day("2026-03-31")));
        assert_eq!(period_end(day("2026-11-01"), months_of("Quarterly"), day("2026-12-15")), Some(day("2026-12-15")), "cut at the lease's last day");
        assert_eq!(months_of("Yearly"), 12);
    }
}
