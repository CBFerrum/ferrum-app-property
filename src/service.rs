//! Service charges. A building's budget for a year (its costs' total) is shared among its units by
//! floor area. A lease that pays service charge is invoiced its unit's share with its rent, each
//! period's line its days in each budget year at that year's rate: the budget in use on its first
//! day, then the next year's for its days past that year when one is in use (else the first's).
//! Once the year is over, the budget is reconciled with what was spent: each such lease's share of
//! what was spent, for its days within the year, against what its invoices bore for those days (each
//! period's line was rounded on its own), is invoiced to its lessee, or credited against its latest
//! invoice of the year that was issued.

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
/// year's budget, the year's days and last day, and the next year's share and days when the period
/// runs into a year with a budget in use.
pub(crate) struct Service {
    pub item: Uuid,
    pub annual: Decimal,
    pub year_days: i64,
    pub year_to: NaiveDate,
    pub next: Option<(Decimal, i64)>,
}

impl Service {
    /// The charge from `from` to `to`: its days in this budget's year at its rate, the days after at
    /// the next year's when it has a budget in use, else at this one's.
    pub(crate) fn for_span(&self, from: NaiveDate, to: NaiveDate) -> Decimal {
        let end = to.min(self.year_to);
        let first = Decimal::from(((end - from).num_days() + 1).max(0));
        let rest = Decimal::from((to - end).num_days().max(0));
        let (annual, days) = self.next.unwrap_or((self.annual, self.year_days));
        round(self.annual * first / Decimal::from(self.year_days.max(1)) + annual * rest / Decimal::from(days.max(1)), 2)
    }
}

/// The service charge of a lease's period from `from` to `until`, when the lease pays one and its
/// building has a budget in use on its first day.
pub(crate) async fn service_for(tx: &mut Tx, tenant: Uuid, lease: &Doc, from: NaiveDate, until: NaiveDate) -> Result<Option<Service>> {
    let Some(unit) = id(lease, "unit").filter(|_| b(lease, "pays_service_charge")) else { return Ok(None) };
    let Some((item, annual, year_from, year_to)) = share_on(tx, tenant, unit, from).await? else { return Ok(None) };
    let next = match year_to.succ_opt().filter(|_| until > year_to) {
        Some(on) => share_on(tx, tenant, unit, on).await?.map(|(_, a, f, t)| (a, (t - f).num_days() + 1)),
        None => None,
    };
    Ok(Some(Service { item, annual, year_days: (year_to - year_from).num_days() + 1, year_to, next }))
}

/// A unit's share of the budget in use on `on` by its basis (floor area, equal shares, or floor area
/// by the units' weightings), with the item it is invoiced as and the budget's year: (item, share,
/// first day, last day).
async fn share_on(tx: &mut Tx, tenant: Uuid, unit: Uuid, on: NaiveDate) -> Result<Option<(Uuid, Decimal, NaiveDate, NaiveDate)>> {
    let row: Option<(Option<Uuid>, Option<Decimal>, NaiveDate, NaiveDate, Option<Decimal>, Option<Decimal>)> = sqlx::query_as(
        "select (select service_charge_item from e_property_settings where tenant_id = $1 limit 1), g.budget, g.year_from, g.year_to, \
           case g.basis when 'Equal Shares' then 1::numeric when 'Weighted Area' then u.area * coalesce(u.weighting, 1) else u.area end, \
           (select sum(case g.basis when 'Equal Shares' then 1::numeric when 'Weighted Area' then o.area * coalesce(o.weighting, 1) else o.area end) \
            from e_unit o where o.tenant_id = g.tenant_id and o.building = g.building) \
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
    Ok(Some((item, budget * area / total, from, to)))
}

/// A lease's balance for a year of `year_days` days it held `days` of: (due, difference), its unit's
/// `share` of `actual` against what it was `charged`, to the cent.
pub(crate) fn balance(charged: Decimal, actual: Decimal, share: Decimal, days: i64, year_days: i64) -> (Decimal, Decimal) {
    let due = round(actual * share * Decimal::from(days) / Decimal::from(year_days.max(1)), 2);
    (due, due - charged)
}

/// What a lease was charged for service over a year: the `item` lines its rent invoices bore, less
/// what their credit notes took back, each for its period's days within the year. Cancelled
/// invoices bear nothing.
async fn charged(tx: &mut Tx, tenant: Uuid, lease: Uuid, item: Uuid, (from, to): (NaiveDate, NaiveDate)) -> Result<Decimal> {
    let sum: Option<Decimal> = sqlx::query_scalar(
        "select sum(i.amount * (least(r.period_to, $5) - greatest(r.period_from, $4) + 1) / (r.period_to - r.period_from + 1)) \
         from e_rent_invoice r join e_sales_invoice si on si.tenant_id = r.tenant_id and si.id in (r.sales_invoice, r.credit_note) \
         join e_sales_invoice__items i on i.tenant_id = si.tenant_id and i.parent_id = si.id and i.item = $3 \
         where r.tenant_id = $1 and r.lease = $2 and si.docstatus < 2 and r.period_from <= $5 and r.period_to >= $4",
    )
    .bind(tenant)
    .bind(lease)
    .bind(item)
    .bind(from)
    .bind(to)
    .fetch_one(&mut **tx)
    .await?;
    Ok(round(sum.unwrap_or_default(), 2))
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

    async fn validate_change(&self, hc: &mut HookCtx<'_>, old: Option<&Doc>, doc: &Doc) -> Result<()> {
        // A budget in use keeps its year and costs: what was invoiced stands on them.
        if let Some(old) = old
            && s(old, "status") != Some("Draft")
            && ["building", "year_from", "year_to", "heads", "budget", "basis"].iter().any(|f| old.get(*f) != doc.get(*f))
        {
            return Err(Error::invalid("a budget in use keeps its building, year, costs and basis"));
        }
        if let Some(cc) = id(doc, "cost_center") {
            let owners: Option<String> = sqlx::query_scalar("select building_name from e_building where tenant_id = $1 and cost_center = $2 limit 1")
                .bind(hc.ctx.tenant)
                .bind(cc)
                .fetch_optional(&mut **hc.tx)
                .await?;
            if let Some(b) = owners {
                return Err(Error::field("cost_center", format!("holds {b}'s owner's costs: shared costs are kept apart")));
            }
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

/// What the submitted bills charged to `cc` and posted in the year came to (their lines' net).
async fn spent_in_books(tx: &mut Tx, tenant: Uuid, cc: Uuid, (from, to): (NaiveDate, NaiveDate)) -> Result<Decimal> {
    Ok(sqlx::query_scalar(
        "select coalesce(sum(coalesce(it.base_net_amount, it.net_amount, it.amount, 0)), 0) from e_purchase_invoice__items it \
         join e_purchase_invoice pi on pi.tenant_id = it.tenant_id and pi.id = it.parent_id \
         where it.tenant_id = $1 and it.cost_center = $2 and pi.docstatus = 1 and pi.posting_date between $3 and $4",
    )
    .bind(tenant)
    .bind(cc)
    .bind(from)
    .bind(to)
    .fetch_one(&mut **tx)
    .await?)
}

/// `reconcile` `{actual?}`: once the year is over, each lease that paid service charge in it is
/// invoiced or credited its unit's share (by the budget's basis) of what was spent less what was
/// budgeted, for its days. What was spent is read from the bills charged to the budget's cost
/// center when it is not given.
async fn reconcile(hc: &mut HookCtx<'_>, doc: &Doc, params: &Value) -> Result<Value> {
    let me = id(doc, "id").ok_or_else(|| Error::internal("a budget without an id"))?;
    let (Some(building), Some(year_from), Some(year_to)) = (id(doc, "building"), date(doc, "year_from"), date(doc, "year_to")) else {
        return Err(Error::invalid("a budget names its building and year"));
    };
    let given = match params.get("actual") {
        None | Some(Value::Null) => None,
        Some(Value::String(v)) if v.trim().is_empty() => None,
        Some(v) => Some(
            v.as_str()
                .map(str::to_string)
                .or_else(|| v.as_f64().map(|f| f.to_string()))
                .and_then(|v| v.trim().parse::<Decimal>().ok())
                .filter(|a| *a >= Decimal::ZERO)
                .ok_or_else(|| Error::field("actual", "is what was spent, nothing or more"))?,
        ),
    };
    let actual = match (given, id(doc, "cost_center")) {
        (Some(a), _) => a,
        (None, Some(cc)) => spent_in_books(hc.tx, hc.ctx.tenant, cc, (year_from, year_to)).await?,
        (None, None) => return Err(Error::field("actual", "is what was spent: give it, or name the budget's cost center to read it from the bills")),
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
    // Each unit's weight by the budget's basis.
    let basis = s(doc, "basis").unwrap_or("Floor Area").to_string();
    let weight = "case $3 when 'Equal Shares' then 1::numeric when 'Weighted Area' then area * coalesce(weighting, 1) else area end";
    let total: Option<Decimal> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("select sum({weight}) from e_unit where tenant_id = $1 and building = $2")))
        .bind(hc.ctx.tenant)
        .bind(building)
        .bind(&basis)
        .fetch_one(&mut **hc.tx)
        .await?;
    let total = total.filter(|t| *t > Decimal::ZERO).ok_or_else(|| Error::invalid("the building's units have no floor area to share by"))?;
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
        let mine: Option<Decimal> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!("select {weight} from e_unit where tenant_id = $1 and id = $2")))
            .bind(hc.ctx.tenant)
            .bind(unit)
            .bind(&basis)
            .fetch_one(&mut **hc.tx)
            .await?;
        let share = mine.unwrap_or_default() / total;
        let days = (last.min(year_to) - start.max(year_from)).num_days() + 1;
        if days <= 0 {
            continue;
        }
        let charged = charged(hc.tx, hc.ctx.tenant, lease_id, item, (year_from, year_to)).await?;
        let (due, difference) = balance(charged, actual, share, days, year_days);
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
                None => (None, Some(format!("{:.2} owed back: no invoice of the year was issued to credit against", -difference))),
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
        // A 60% share for the whole year, charged 6570, 9855 spent: 657 back.
        assert_eq!(balance(n("6570"), n("9855"), n("0.6"), 365, 365), (n("5913"), n("-657")));
        // A 40% share for 184 days, charged 2208: 1987.20 due.
        assert_eq!(balance(n("2208"), n("9855"), n("0.4"), 184, 365), (n("1987.2"), n("-220.8")));
        // More spent than budgeted: owed.
        assert_eq!(balance(n("6570"), n("12045"), n("0.6"), 365, 365).1, n("657"));
        // Twelve months each rounded bore a cent more than a third of 1000: a cent back.
        assert_eq!(balance(n("333.34"), n("1000"), n("1") / n("3"), 365, 365), (n("333.33"), n("-0.01")));
        let day = |v: &str| NaiveDate::parse_from_str(v, "%Y-%m-%d").unwrap();
        let sv = Service { item: Uuid::nil(), annual: n("6570"), year_days: 365, year_to: day("2025-12-31"), next: None };
        let span = |s: &Service, from: &str, to: &str| s.for_span(day(from), day(to));
        assert_eq!(
            (span(&sv, "2025-01-01", "2025-01-31"), span(&sv, "2025-02-01", "2025-02-28"), span(&sv, "2025-01-01", "2025-12-31")),
            (n("558"), n("504"), n("6570")),
            "18 a day"
        );
        // Across the year's end: December at 18 a day, then January at the next year's 20 a day, or
        // at 18 when the next year has no budget in use.
        let both = Service { next: Some((n("7300"), 365)), ..sv };
        assert_eq!(span(&both, "2025-12-01", "2026-01-31"), n("1178"));
        assert_eq!(span(&Service { next: None, ..both }, "2025-12-01", "2026-01-31"), n("1116"));
    }
}
