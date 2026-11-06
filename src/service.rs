//! Service charges. A building's budget for a year (its costs' total) is shared among its units by
//! floor area. A lease that pays service charge is invoiced its unit's share with its rent, each
//! period's line the period's days of the year at the rate of the budget in use on its first day.
//! Once the year is over, the budget is reconciled with what was spent: each such lease's share of
//! the difference, for its days within the year, is invoiced to its lessee, or credited against its
//! latest invoice of the year that was issued.

use crate::leases::last_day;
use crate::{date, obj};
use async_trait::async_trait;
use chrono::NaiveDate;
use ferrum_domain::util::{b, d, elevated, id, money, round, rows, s};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result, Tx};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

/// The longest year a budget covers, in days.
const LONGEST_YEAR: i64 = 400;

/// What a lease's period bears for service: the item it is invoiced as, the unit's share of the
/// year's budget, and the year's days.
pub(crate) struct Service {
    pub item: Uuid,
    pub annual: Decimal,
    pub year_days: i64,
}

impl Service {
    /// A period of `days` days' charge.
    pub(crate) fn for_days(&self, days: i64) -> Decimal {
        round(self.annual * Decimal::from(days) / Decimal::from(self.year_days.max(1)), 2)
    }
}

/// The service charge of a lease's period starting `on`, when the lease pays one and its building
/// has a budget in use on that day.
pub(crate) async fn service_for(tx: &mut Tx, tenant: Uuid, lease: &Doc, on: NaiveDate) -> Result<Option<Service>> {
    let Some(unit) = id(lease, "unit").filter(|_| b(lease, "pays_service_charge")) else { return Ok(None) };
    let row: Option<(Option<Uuid>, Option<Decimal>, NaiveDate, NaiveDate, Option<Decimal>, Option<Decimal>)> = sqlx::query_as(
        "select (select service_charge_item from e_property_settings where tenant_id = $1 limit 1), g.budget, g.year_from, g.year_to, u.area, \
           (select sum(o.area) from e_unit o where o.tenant_id = g.tenant_id and o.building = g.building) \
         from e_service_charge_budget g join e_unit u on u.tenant_id = g.tenant_id and u.id = $2 and u.building = g.building \
         where g.tenant_id = $1 and g.status in ('Active', 'Reconciled') and $3 between g.year_from and g.year_to order by g.year_from desc limit 1",
    )
    .bind(tenant)
    .bind(unit)
    .bind(on)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((Some(item), Some(budget), from, to, Some(area), Some(total))) = row else { return Ok(None) };
    if total <= Decimal::ZERO {
        return Ok(None);
    }
    Ok(Some(Service { item, annual: budget * area / total, year_days: (to - from).num_days() + 1 }))
}

/// A lease's balance for a year of `year_days` days it held `days` of: (charged, due, difference),
/// its unit's `share` of `budget` against `actual`, to the cent.
pub(crate) fn balance(budget: Decimal, actual: Decimal, share: Decimal, days: i64, year_days: i64) -> (Decimal, Decimal, Decimal) {
    let part = |total: Decimal| round(total * share * Decimal::from(days) / Decimal::from(year_days.max(1)), 2);
    let (charged, due) = (part(budget), part(actual));
    (charged, due, due - charged)
}

pub struct BudgetHook;

#[async_trait]
impl DocHook for BudgetHook {
    fn name(&self) -> &'static str {
        "property.service_charge_budget"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["budget"]
    }

    async fn before_validate(&self, _hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        let total: Decimal = rows(doc, "heads").iter().map(|h| d(h, "amount")).sum();
        doc.insert("budget".into(), json!(money(total)));
        Ok(())
    }

    async fn validate(&self, _hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        let (Some(from), Some(to)) = (date(doc, "year_from"), date(doc, "year_to")) else { return Ok(()) };
        if to < from || (to - from).num_days() + 1 > LONGEST_YEAR {
            return Err(Error::field("year_to", format!("is after its start, within {LONGEST_YEAR} days")));
        }
        Ok(())
    }

    async fn validate_change(&self, _hc: &mut HookCtx<'_>, old: Option<&Doc>, doc: &Doc) -> Result<()> {
        // A budget in use keeps its year and costs: what was invoiced stands on them.
        if let Some(old) = old
            && s(old, "status") != Some("Draft")
            && ["building", "year_from", "year_to", "heads", "budget"].iter().any(|f| old.get(*f) != doc.get(*f))
        {
            return Err(Error::invalid("a budget in use keeps its building, year and costs"));
        }
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, params: &Value) -> Result<Option<Value>> {
        let me = id(doc, "id").ok_or_else(|| Error::internal("a budget without an id"))?;
        let system = elevated(hc.ctx);
        match name {
            "activate_budget" => {
                let (Some(building), Some(from), Some(to)) = (id(doc, "building"), date(doc, "year_from"), date(doc, "year_to")) else {
                    return Err(Error::invalid("a budget names its building and year"));
                };
                let other: Option<NaiveDate> = sqlx::query_scalar(
                    "select year_from from e_service_charge_budget where tenant_id = $1 and building = $2 and id <> $3 and status in ('Active', 'Reconciled') \
                     and year_from <= $5 and year_to >= $4 limit 1",
                )
                .bind(hc.ctx.tenant)
                .bind(building)
                .bind(me)
                .bind(from)
                .bind(to)
                .fetch_optional(&mut **hc.tx)
                .await?;
                if let Some(other) = other {
                    return Err(Error::invalid(format!("the building has a budget in use from {other} over some of these days")));
                }
                hc.kernel.set_values_tx(&system, hc.tx, "ServiceChargeBudget", me, obj(json!({"status": "Active"}))).await?;
                Ok(Some(json!({"status": "Active"})))
            }
            "reconcile" => Ok(Some(reconcile(hc, doc, params).await?)),
            _ => Ok(None),
        }
    }
}

/// `reconcile` `{actual}`: once the year is over, each lease that paid service charge in it is
/// invoiced or credited its unit's share of what was spent less what was budgeted, for its days.
async fn reconcile(hc: &mut HookCtx<'_>, doc: &Doc, params: &Value) -> Result<Value> {
    let me = id(doc, "id").ok_or_else(|| Error::internal("a budget without an id"))?;
    let actual: Decimal = params
        .get("actual")
        .and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_f64().map(|f| f.to_string())))
        .and_then(|v| v.parse().ok())
        .filter(|a: &Decimal| *a >= Decimal::ZERO)
        .ok_or_else(|| Error::field("actual", "is what was spent, nothing or more"))?;
    let (Some(building), Some(year_from), Some(year_to)) = (id(doc, "building"), date(doc, "year_from"), date(doc, "year_to")) else {
        return Err(Error::invalid("a budget names its building and year"));
    };
    if hc.ctx.today() <= year_to {
        return Err(Error::field("actual", format!("is known once its year is over, after {year_to}")));
    }
    let settings: Option<(Option<Uuid>, Option<Uuid>)> =
        sqlx::query_as("select company, service_charge_item from e_property_settings where tenant_id = $1 limit 1")
            .bind(hc.ctx.tenant)
            .fetch_optional(&mut **hc.tx)
            .await?;
    let Some((Some(company), Some(item))) = settings else {
        return Err(Error::invalid("the settings name no company and service charge item to invoice with"));
    };
    let currency: Option<Uuid> = sqlx::query_scalar("select default_currency from e_company where tenant_id = $1 and id = $2")
        .bind(hc.ctx.tenant)
        .bind(company)
        .fetch_optional(&mut **hc.tx)
        .await?
        .flatten();
    let total: Option<Decimal> = sqlx::query_scalar("select sum(area) from e_unit where tenant_id = $1 and building = $2")
        .bind(hc.ctx.tenant)
        .bind(building)
        .fetch_one(&mut **hc.tx)
        .await?;
    let total = total.filter(|t| *t > Decimal::ZERO).ok_or_else(|| Error::invalid("the building's units have no floor area to share by"))?;
    let budget = d(doc, "budget");
    let year_days = (year_to - year_from).num_days() + 1;
    let system = elevated(hc.ctx);
    let leases: Vec<Uuid> = sqlx::query_scalar(
        "select l.id from e_lease l join e_unit u on u.tenant_id = l.tenant_id and u.id = l.unit where l.tenant_id = $1 and u.building = $2 \
         and l.status <> 'Draft' and coalesce(l.pays_service_charge, false) and l.starts_on <= $4 \
         and least(l.ends_on, coalesce(l.terminated_on, l.leaving_on, l.ends_on)) >= $3 order by l.starts_on, l.id",
    )
    .bind(hc.ctx.tenant)
    .bind(building)
    .bind(year_from)
    .bind(year_to)
    .fetch_all(&mut **hc.tx)
    .await?;
    let mut balances = Vec::new();
    for lease_id in leases {
        let lease = hc.kernel.get_tx(&system, hc.tx, "Lease", lease_id, false).await?;
        let (Some(start), Some(last), Some(unit)) = (date(&lease, "starts_on"), last_day(&lease), id(&lease, "unit")) else { continue };
        let area: Option<Decimal> =
            sqlx::query_scalar("select area from e_unit where tenant_id = $1 and id = $2").bind(hc.ctx.tenant).bind(unit).fetch_one(&mut **hc.tx).await?;
        let share = area.unwrap_or_default() / total;
        let days = (last.min(year_to) - start.max(year_from)).num_days() + 1;
        if days <= 0 {
            continue;
        }
        let (charged, due, difference) = balance(budget, actual, share, days, year_days);
        let what = format!("Service charge balance, {year_from} to {year_to}: {days} days of a {}% share", round(share * Decimal::ONE_HUNDRED, 2));
        let (document, note) = if difference > Decimal::ZERO {
            let inv = hc
                .kernel
                .insert_tx(
                    &system,
                    hc.tx,
                    "SalesInvoice",
                    obj(json!({"company": company.to_string(), "customer": s(&lease, "lessee"), "currency": currency.map(|c| c.to_string()), "remarks": what,
                               "items": [{"item": item.to_string(), "qty": "1", "price_list_rate": money(difference), "discount_percent": "0", "description": what}]})),
                )
                .await?;
            (inv.get("id").cloned(), None)
        } else if difference < Decimal::ZERO {
            // Credited against its latest invoice of the year that was issued.
            let against: Option<Uuid> = sqlx::query_scalar(
                "select si.id from e_rent_invoice r join e_sales_invoice si on si.tenant_id = r.tenant_id and si.id = r.sales_invoice \
                 where r.tenant_id = $1 and r.lease = $2 and si.docstatus = 1 and r.period_from between $3 and $4 order by r.period_from desc limit 1",
            )
            .bind(hc.ctx.tenant)
            .bind(lease_id)
            .bind(year_from)
            .bind(year_to)
            .fetch_optional(&mut **hc.tx)
            .await?;
            match against {
                Some(against) => {
                    let note = hc
                        .kernel
                        .insert_tx(
                            &system,
                            hc.tx,
                            "SalesInvoice",
                            obj(json!({"company": company.to_string(), "customer": s(&lease, "lessee"), "currency": currency.map(|c| c.to_string()),
                                       "is_return": true, "return_against": against.to_string(), "remarks": what,
                                       "items": [{"item": item.to_string(), "qty": "-1", "price_list_rate": money(-difference), "discount_percent": "0", "description": what}]})),
                        )
                        .await?;
                    (note.get("id").cloned(), None)
                }
                None => (None, Some(format!("{} owed back: no invoice of the year was issued to credit against", money(-difference)))),
            }
        } else {
            (None, None)
        };
        balances.push(obj(json!({"lease": lease_id.to_string(), "days": days, "share": money(round(share * Decimal::ONE_HUNDRED, 2)),
                                  "charged": money(charged), "due": money(due), "difference": money(difference), "document": document, "note": note})));
    }
    let n = balances.len();
    hc.kernel.set_child_rows_tx(&system, hc.tx, "ServiceChargeBudget", "balances", me, balances).await?;
    hc.kernel.set_values_tx(&system, hc.tx, "ServiceChargeBudget", me, obj(json!({"status": "Reconciled", "actual": money(actual)}))).await?;
    Ok(json!({"status": "Reconciled", "leases": n}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_lease_its_share_of_the_difference() {
        let n = |v: &str| v.parse::<Decimal>().unwrap();
        // A 60% share of a 10950 budget for the whole year, 9855 spent: 657 back.
        assert_eq!(balance(n("10950"), n("9855"), n("0.6"), 365, 365), (n("6570"), n("5913"), n("-657")));
        // A 40% share for 184 days: 2208 charged, 1987.20 due.
        assert_eq!(balance(n("10950"), n("9855"), n("0.4"), 184, 365), (n("2208"), n("1987.2"), n("-220.8")));
        // More spent than budgeted: owed.
        assert_eq!(balance(n("10950"), n("12045"), n("0.6"), 365, 365).2, n("657"));
        let sv = Service { item: Uuid::nil(), annual: n("6570"), year_days: 365 };
        assert_eq!((sv.for_days(31), sv.for_days(28), sv.for_days(365)), (n("558"), n("504"), n("6570")), "18 a day");
    }
}
